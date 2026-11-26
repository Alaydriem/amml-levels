use core::fmt::Write;

use crate::config::{InfoConfig, LabelConfig, LightConfig};
use crate::services::{ActorKinds, EntityCensus, FrameRate, InfoLayout, LightClassifier, MotionMeter, SlimeChunks};
use crate::structs::frame::ColumnRequest;
use crate::structs::info::{
    ActorSample, EntityCounts, InfoConnection, InfoLine, InfoLines, InfoMode, InfoNames, InfoPlacement, InfoSample,
    InfoTarget, InfoWorldFacts,
};
use crate::structs::light::{LightBand, SurfaceSample};
use crate::structs::manager::ManagerSignal;
use crate::structs::world::{BlockPos, ColumnPos, Vec3f};

const CHUNK_SIZE: i32 = 16;
const OVERWORLD: i32 = 0;
const BYTES_PER_MB: f64 = 1024.0 * 1024.0;
const US_PER_MS: f32 = 1000.0;
const FULL_TURN: f32 = 360.0;
const QUARTER_TURN: f32 = 90.0;
const HALF_QUARTER_TURN: f32 = 45.0;
// Bedrock yaw: 0 faces south (+Z) and the angle grows toward west
const FACINGS: [&str; 4] = ["south", "west", "north", "east"];
// the game's block face index
const FACES: [&str; 6] = ["down", "up", "north", "south", "west", "east"];
// an interval read from an f32 setting is a little off in f64; this keeps a refresh at exactly the interval due
const DUE_TOLERANCE: f64 = 1e-6;
// feet that rest on a block top sit at its exact height; this drops them into the block they stand on
const STANDING_DEPTH: f32 = 0.01;

// The F3+T panel: its shown state, its refresh timers and every line it shows. The lines are rebuilt in place at
// the refresh interval into fixed buffers, so a refresh allocates nothing.
pub struct InfoPanel {
    config: InfoConfig,
    classifier: LightClassifier,
    red: [f32; 3],
    yellow: [f32; 3],
    green: [f32; 3],
    census: EntityCensus,
    has_census: bool,
    motion: MotionMeter,
    frame_rate: FrameRate,
    lines: InfoLines,
    world_name: InfoLine,
    has_world_name: bool,
    world_seed: Option<u64>,
    connection: InfoConnection,
    // (used, budget) in bytes
    memory: Option<(u64, u64)>,
    wanted: bool,
    lease_lost: bool,
    last_refresh: Option<f64>,
    last_scan: Option<f64>,
}

impl InfoPanel {
    const TEXT_ALPHA: f32 = 1.0;
    const BACKGROUND_COLOR: [f32; 3] = [0.0, 0.0, 0.0];

    pub fn new(config: &InfoConfig, light: LightConfig, label: &LabelConfig) -> Self {
        Self {
            config: config.clone().sanitized(),
            classifier: LightClassifier::new(light),
            red: Self::rgb(label.red),
            yellow: Self::rgb(label.yellow),
            green: Self::rgb(label.green),
            census: EntityCensus::new(),
            has_census: false,
            motion: MotionMeter::new(),
            frame_rate: FrameRate::new(),
            lines: InfoLines::new(),
            world_name: InfoLine::new(),
            has_world_name: false,
            world_seed: None,
            connection: InfoConnection::Unknown,
            memory: None,
            wanted: false,
            lease_lost: false,
            last_refresh: None,
            last_scan: None,
        }
    }

    pub fn apply_config(&mut self, config: &InfoConfig, light: LightConfig, label: &LabelConfig) {
        self.config = config.clone().sanitized();
        self.classifier = LightClassifier::new(light);
        self.red = Self::rgb(label.red);
        self.yellow = Self::rgb(label.yellow);
        self.green = Self::rgb(label.green);
    }

    pub fn signal(&mut self, signal: ManagerSignal) {
        let was_shown = self.is_shown();
        let is_hold = self.config.mode == InfoMode::Hold;
        match signal {
            ManagerSignal::InfoPressed => self.wanted = is_hold || !self.wanted,
            ManagerSignal::InfoReleased => {
                if is_hold {
                    self.wanted = false;
                }
            }
            ManagerSignal::HudLeaseLost => {
                self.lease_lost = true;
                if is_hold {
                    self.wanted = false;
                }
            }
            ManagerSignal::HudLeaseRestored => self.lease_lost = false,
            ManagerSignal::Disabled => {
                self.wanted = false;
                self.lease_lost = false;
            }
            ManagerSignal::TogglePressed
            | ManagerSignal::BordersPressed
            | ManagerSignal::SphereDropPressed
            | ManagerSignal::SphereRemovePressed
            | ManagerSignal::SphereClearPressed
            | ManagerSignal::LeaseLost
            | ManagerSignal::LeaseSuspended
            | ManagerSignal::LeaseRestored => {}
        }
        if !was_shown && self.is_shown() {
            self.last_refresh = None;
            self.last_scan = None;
        }
    }

    // hides the panel and drops everything that belongs to the world that ended
    pub fn leave_world(&mut self) {
        self.wanted = false;
        self.lines.clear();
        self.motion.reset();
        self.census.submit(&[]);
        self.has_census = false;
        self.frame_rate = FrameRate::new();
        self.world_name.clear();
        self.has_world_name = false;
        self.world_seed = None;
        self.connection = InfoConnection::Unknown;
        self.last_refresh = None;
        self.last_scan = None;
    }

    // a new world or dimension: the lines and the census of the old place go, and a refresh and a scan are due at once
    pub fn place_changed(&mut self) {
        self.lines.clear();
        self.census.submit(&[]);
        self.has_census = false;
        self.last_refresh = None;
        self.last_scan = None;
    }

    pub fn is_shown(&self) -> bool {
        self.wanted && !self.lease_lost
    }

    pub fn mode(&self) -> InfoMode {
        self.config.mode
    }

    // the counts of the last entity scan; None before the first scan of this world
    pub fn entity_counts(&self) -> Option<&EntityCounts> {
        self.has_census.then(|| self.census.counts())
    }

    // a lost lease does not drop the wish to show, so a suspended lease is kept and comes back
    pub fn wants_hud_lease(&self) -> bool {
        self.wanted
    }

    pub fn refresh_due(&self, now: f64) -> bool {
        self.is_shown() && Self::due(self.last_refresh, now, self.config.refresh_seconds)
    }

    pub fn entity_scan_due(&self, now: f64) -> bool {
        self.is_shown() && Self::due(self.last_scan, now, self.config.entity_scan_seconds)
    }

    pub fn entity_radius(&self) -> f32 {
        self.config.entity_radius
    }

    // the block the feet are in
    pub fn feet_block(&self, position: Vec3f) -> BlockPos {
        Self::block_of(position)
    }

    // the surface the light line reads: the one under the block the feet are in
    pub fn feet_scan_range(&self, position: Vec3f) -> ColumnRequest {
        let block = Self::block_of(position);
        ColumnRequest { column: ColumnPos { x: block.x, z: block.z }, y_min: block.y - 1, y_max: block.y - 1 }
    }

    // the box the entity lines count in: (min, max), the entity radius around the position on each axis
    pub fn entity_scan_box(&self, position: Vec3f) -> (Vec3f, Vec3f) {
        let radius = self.config.entity_radius;
        let min = Vec3f { x: position.x - radius, y: position.y - radius, z: position.z - radius };
        let max = Vec3f { x: position.x + radius, y: position.y + radius, z: position.z + radius };
        (min, max)
    }

    pub fn motion(&mut self, pos: Vec3f, dimension: i32, now: f64) {
        self.motion.sample(pos, dimension, now);
    }

    pub fn frames(&mut self, total_frames: u64, now: f64) {
        self.frame_rate.sample(total_frames, now);
    }

    pub fn frame_cost(&mut self, us: f32) {
        self.frame_rate.cost(us);
    }

    pub fn levels_memory(&mut self, used: u64, budget: u64) {
        self.memory = Some((used, budget));
    }

    pub fn submit_entities(&mut self, actors: &[ActorSample], now: f64) {
        self.census.submit(actors);
        self.has_census = true;
        self.last_scan = Some(now);
    }

    // the game cannot count entities now: the last census is dropped and the entity lines go away until a scan comes
    pub fn census_unavailable(&mut self) {
        self.census.submit(&[]);
        self.has_census = false;
    }

    pub fn set_world(&mut self, facts: InfoWorldFacts<'_>) {
        self.world_name.clear();
        if let Some(name) = facts.level_name {
            let _ = self.world_name.write_str(name);
        }
        self.has_world_name = facts.level_name.is_some();
        self.world_seed = facts.seed;
        self.connection = facts.connection;
    }

    pub fn refresh(&mut self, sample: &InfoSample, names: InfoNames<'_>, feet_column: &[SurfaceSample]) {
        self.last_refresh = Some(sample.time_seconds);
        self.lines.clear();
        let block = Self::block_of(sample.position);
        if self.config.show_position {
            self.position_lines(sample);
        }
        if self.config.show_world {
            self.world_line(sample, names);
        }
        if self.config.show_chunk && sample.dimension == OVERWORLD {
            self.slime_line(block);
        }
        if self.config.show_light {
            self.light_line(sample, block, feet_column);
        }
        if self.config.show_target {
            self.target_line(sample, names);
        }
        if self.config.show_motion {
            self.speed_line();
        }
        if self.config.show_entities && self.has_census {
            self.entity_lines();
        }
        if self.config.show_performance {
            self.performance_line();
        }
        if self.config.show_amml {
            self.world_facts_line();
            self.levels_line();
        }
    }

    pub fn lines(&self) -> &InfoLines {
        &self.lines
    }

    pub fn placement(&self, screen_w: f32, screen_h: f32, widest: f32) -> InfoPlacement {
        InfoLayout::place(screen_w, screen_h, widest, self.lines.len(), self.config.corner, self.config.top_offset)
    }

    pub fn background_alpha(&self) -> f32 {
        self.config.background_alpha
    }

    pub fn background_color(&self) -> [f32; 3] {
        Self::BACKGROUND_COLOR
    }

    pub fn text_alpha(&self) -> f32 {
        Self::TEXT_ALPHA
    }

    // the panel's buffers, all made in new()
    pub fn resident_bytes(&self) -> u64 {
        (std::mem::size_of::<Self>() + InfoLines::MAX_LINES * std::mem::size_of::<InfoLine>()) as u64
    }

    fn position_lines(&mut self, sample: &InfoSample) {
        let p = sample.position;
        let block = Self::block_of(Vec3f { y: p.y - STANDING_DEPTH, ..p });
        if let Some(line) = self.lines.next_line() {
            let _ = write!(line, "XYZ: {:.3} / {:.3} / {:.3}", p.x, p.y, p.z);
        }
        if let Some(line) = self.lines.next_line() {
            let _ = write!(
                line,
                "Standing on: {} {} {}  Chunk: {} {} in {} {}",
                block.x,
                block.y,
                block.z,
                block.x.div_euclid(CHUNK_SIZE),
                block.z.div_euclid(CHUNK_SIZE),
                block.x.rem_euclid(CHUNK_SIZE),
                block.z.rem_euclid(CHUNK_SIZE),
            );
        }
        if let Some(line) = self.lines.next_line() {
            let _ =
                write!(line, "Facing: {} (yaw {:.1} pitch {:.1})", Self::facing(sample.yaw), sample.yaw, sample.pitch);
        }
    }

    fn world_line(&mut self, sample: &InfoSample, names: InfoNames<'_>) {
        let Some(line) = self.lines.next_line() else {
            return;
        };
        let _ = line.write_str("Dimension: ");
        match sample.dimension {
            0 => line.write_str("overworld"),
            1 => line.write_str("nether"),
            2 => line.write_str("the_end"),
            other => write!(line, "#{other}"),
        }
        .ok();
        match (names.biome, sample.biome) {
            (Some(name), _) => write!(line, "  Biome: {name}"),
            (None, Some(id)) => write!(line, "  Biome: #{id}"),
            (None, None) => Ok(()),
        }
        .ok();
    }

    fn slime_line(&mut self, block: BlockPos) {
        let is_slime = SlimeChunks::is_slime(block.x.div_euclid(CHUNK_SIZE), block.z.div_euclid(CHUNK_SIZE));
        let green = self.green;
        let Some(line) = self.lines.next_line() else {
            return;
        };
        if is_slime {
            let _ = line.write_str("Slime chunk: yes");
            line.set_color(green);
        } else {
            let _ = line.write_str("Slime chunk: no");
        }
    }

    // the surface under the feet holds the light of the block the player stands in
    fn light_line(&mut self, sample: &InfoSample, block: BlockPos, feet_column: &[SurfaceSample]) {
        let Some(surface) = feet_column.iter().find(|surface| surface.pos.y == block.y - 1) else {
            return;
        };
        let light = LightClassifier::light(surface.block_light, surface.sky_light, sample.sky_darken);
        let color = match self.classifier.classify(light) {
            LightBand::Red => self.red,
            LightBand::Yellow => self.yellow,
            LightBand::Green => self.green,
        };
        let Some(line) = self.lines.next_line() else {
            return;
        };
        let _ = write!(line, "Light: {} (block {} sky {})", light, surface.block_light, surface.sky_light);
        line.set_color(color);
    }

    fn target_line(&mut self, sample: &InfoSample, names: InfoNames<'_>) {
        match sample.target {
            InfoTarget::None => {}
            InfoTarget::Block { pos, face } => {
                let Some(line) = self.lines.next_line() else {
                    return;
                };
                let name = names.target_block.unwrap_or("block");
                let _ = write!(line, "Target: {} {} {} {} face ", name, pos.x, pos.y, pos.z);
                match FACES.get(face as usize) {
                    Some(face_name) => line.write_str(face_name),
                    None => write!(line, "#{face}"),
                }
                .ok();
            }
            InfoTarget::Entity { point } => {
                let kind = self.census.kind_at(point);
                let distance = Self::distance(sample.position, point);
                let Some(line) = self.lines.next_line() else {
                    return;
                };
                let _ = line.write_str("Target: ");
                match kind {
                    Some(actor_type) => Self::write_kind(line, actor_type),
                    None => {
                        let _ = line.write_str("entity");
                    }
                }
                let _ = write!(line, " {distance:.1} m");
            }
        }
    }

    fn speed_line(&mut self) {
        let Some((horizontal, total)) = self.motion.speed() else {
            return;
        };
        if let Some(line) = self.lines.next_line() {
            let _ = write!(line, "Speed: {total:.2} b/s (horizontal {horizontal:.2})");
        }
    }

    fn entity_lines(&mut self) {
        let counts = *self.census.counts();
        if let Some(line) = self.lines.next_line() {
            let _ = write!(
                line,
                "Entities: {}  players {}  hostile {}  passive {}  other {}",
                counts.total, counts.players, counts.hostile, counts.passive, counts.other
            );
        }
        if counts.top_hostile.iter().all(|&(_, count)| count == 0) {
            return;
        }
        let Some(line) = self.lines.next_line() else {
            return;
        };
        let _ = line.write_str("Hostile:");
        let mut separator = " ";
        for &(kind, count) in counts.top_hostile.iter().filter(|&&(_, count)| count > 0) {
            let _ = line.write_str(separator);
            Self::write_kind(line, kind as u32);
            let _ = write!(line, " {count}");
            separator = "  ";
        }
    }

    fn performance_line(&mut self) {
        let Some(fps) = self.frame_rate.fps() else {
            return;
        };
        let frame_ms = self.frame_rate.frame_ms();
        let Some(line) = self.lines.next_line() else {
            return;
        };
        let _ = write!(line, "FPS: {fps:.0}");
        if let Some(ms) = frame_ms {
            let _ = write!(line, "  Frame: {ms:.1} ms");
        }
    }

    fn world_facts_line(&mut self) {
        let connection = match self.connection {
            InfoConnection::Local => Some("Local"),
            InfoConnection::Realm => Some("Realm"),
            InfoConnection::Server => Some("Server"),
            InfoConnection::Unknown => None,
        };
        if !self.has_world_name && self.world_seed.is_none() && connection.is_none() {
            return;
        }
        let Some(line) = self.lines.next_line() else {
            return;
        };
        let mut separator = "";
        if self.has_world_name {
            let _ = write!(line, "World: {}", self.world_name.as_str());
            separator = "  ";
        }
        if let Some(seed) = self.world_seed {
            let _ = write!(line, "{separator}Seed: {seed}");
            separator = "  ";
        }
        if let Some(connection) = connection {
            let _ = write!(line, "{separator}{connection}");
        }
    }

    fn levels_line(&mut self) {
        let cost_us = self.frame_rate.cost_us();
        let memory = self.memory;
        if cost_us.is_none() && memory.is_none() {
            return;
        }
        let Some(line) = self.lines.next_line() else {
            return;
        };
        let _ = line.write_str("Levels:");
        let mut separator = " ";
        if let Some(us) = cost_us {
            let _ = write!(line, "{separator}{:.2} ms", us / US_PER_MS);
            separator = "  ";
        }
        if let Some((used, budget)) = memory {
            let used_mb = used as f64 / BYTES_PER_MB;
            let budget_mb = budget as f64 / BYTES_PER_MB;
            let _ = write!(line, "{separator}{used_mb:.1} / {budget_mb:.0} MB");
        }
    }

    // the vanilla name of the kind, or `#` and the kind byte when the name table has no entry
    fn write_kind(line: &mut InfoLine, actor_type: u32) {
        match ActorKinds::name(actor_type) {
            Some(name) => line.write_str(name),
            None => write!(line, "#{}", actor_type & 0xFF),
        }
        .ok();
    }

    fn due(last: Option<f64>, now: f64, interval: f32) -> bool {
        match last {
            Some(last) => now - last >= interval as f64 - DUE_TOLERANCE,
            None => true,
        }
    }

    fn block_of(position: Vec3f) -> BlockPos {
        BlockPos { x: position.x.floor() as i32, y: position.y.floor() as i32, z: position.z.floor() as i32 }
    }

    fn facing(yaw: f32) -> &'static str {
        let turn = (yaw.rem_euclid(FULL_TURN) + HALF_QUARTER_TURN) / QUARTER_TURN;
        FACINGS[(turn.floor() as usize) % FACINGS.len()]
    }

    fn distance(from: Vec3f, to: Vec3f) -> f32 {
        let (dx, dy, dz) = (to.x - from.x, to.y - from.y, to.z - from.z);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    fn rgb(color: [f32; 4]) -> [f32; 3] {
        [color[0], color[1], color[2]]
    }
}
