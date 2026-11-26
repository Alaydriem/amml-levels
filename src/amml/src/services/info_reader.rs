use amml_api::amml_abi::AmmlFace;
use amml_api::{
    Aabb, ActorEntry, Area, Biomes, BlockPos, BlockSource, Client, ColumnPos, ConnectionKind, Ctx, HitResult,
    SurfaceSample, Vec3, WorldInfo,
};
use levels_core::services::InfoPanel;
use levels_core::structs::frame::ColumnRequest;
use levels_core::structs::info::{ActorSample, InfoConnection, InfoNames, InfoSample, InfoTarget, InfoWorldFacts};
use levels_core::structs::light::SurfaceSample as CoreSample;
use levels_core::structs::world::{BlockPos as CoreBlockPos, Vec3f};

use crate::services::BiomeNames;

const MAX_ACTORS: usize = BlockSource::<'static>::MAX_ACTORS;
const FEET_SURFACES: usize = 32;

// the game reads of the info panel on the client tick: it reads what the panel asks for when it asks, and forwards it.
// Every buffer is made in new(); a tick allocates only for a biome name new to the cache, a new target block name and
// the world facts of a new world or dimension
pub struct InfoReader {
    ctx: Ctx,
    biomes: Biomes,
    names: BiomeNames,
    entries: [ActorEntry; MAX_ACTORS],
    actors: [ActorSample; MAX_ACTORS],
    found: [SurfaceSample; FEET_SURFACES],
    feet_column: [CoreSample; FEET_SURFACES],
    target_name: String,
    has_target_name: bool,
    // the block the target name was read for
    target_pos: Option<CoreBlockPos>,
    // (in a world, dimension) of the last tick that could read them
    last_place: Option<(bool, i32)>,
    // whether the world facts of the current place came from the game
    facts_read: bool,
}

impl InfoReader {
    const TARGET_NAME_BYTES: usize = 96;

    pub fn new(ctx: Ctx) -> Self {
        Self {
            ctx,
            biomes: ctx.biomes(),
            names: BiomeNames::new(),
            entries: [ActorEntry::default(); MAX_ACTORS],
            actors: [ActorSample::default(); MAX_ACTORS],
            found: [SurfaceSample::default(); FEET_SURFACES],
            feet_column: [CoreSample::default(); FEET_SURFACES],
            target_name: String::with_capacity(Self::TARGET_NAME_BYTES),
            has_target_name: false,
            target_pos: None,
            last_place: None,
            facts_read: false,
        }
    }

    // whether an entity scan went to core in this tick
    pub fn tick(&mut self, client: &Client<'_>, panel: &mut InfoPanel, now: f64) -> bool {
        let in_world = client.world_active() == Some(true);
        let player = if in_world { client.local_player() } else { None };
        let dimension = player.as_ref().and_then(|player| player.dimension());
        let dimension_id = dimension.as_ref().and_then(|dimension| dimension.id());
        let place = if in_world { dimension_id.map(|id| (true, id)) } else { Some((false, 0)) };
        if let Some(place) = place
            && self.last_place != Some(place)
        {
            self.last_place = Some(place);
            self.names.clear();
            self.target_pos = None;
            self.has_target_name = false;
            panel.place_changed();
            self.facts_read = Self::read_world(client.world_info(), panel);
        } else if in_world && !self.facts_read {
            self.facts_read = Self::read_world(client.world_info(), panel);
        }
        if !panel.is_shown() {
            return false;
        }
        let (Some(player), Some(dimension_id)) = (player, dimension_id) else { return false };
        let Some(feet) = player.position() else { return false };
        let position = Vec3f { x: feet.x, y: feet.y, z: feet.z };
        panel.motion(position, dimension_id, now);
        let region = player.block_source();
        let mut scanned = false;
        if panel.entity_scan_due(now) {
            // a loader that does not serve the actor entries now gives no census, so core hides the entity lines
            if !self.ctx.supports(Area::WORLD_ACTORS_IN.at(2)) {
                panel.census_unavailable();
            } else if let Some(region) = &region {
                self.scan_entities(region, position, panel, now);
                scanned = true;
            }
        }
        if !panel.refresh_due(now) {
            return scanned;
        }
        let request = panel.feet_scan_range(position);
        let feet = panel.feet_block(position);
        let feet_block = BlockPos { x: feet.x, y: feet.y, z: feet.z };
        let biome = region.as_ref().and_then(|region| region.biome_at(feet_block));
        let count = region.as_ref().map_or(0, |region| self.read_feet_column(region, request));
        let target = match player.hit_result() {
            Some(HitResult::Block(hit)) => InfoTarget::Block {
                pos: CoreBlockPos { x: hit.pos.x, y: hit.pos.y, z: hit.pos.z },
                face: AmmlFace::from(hit.face).0,
            },
            Some(HitResult::Entity { point }) => {
                InfoTarget::Entity { point: Vec3f { x: point.x, y: point.y, z: point.z } }
            }
            _ => InfoTarget::None,
        };
        if let InfoTarget::Block { pos, .. } = target
            && self.target_pos != Some(pos)
        {
            self.target_pos = Some(pos);
            self.read_target_name(region.as_ref(), pos);
        }
        let rotation = player.rotation();
        let sample = InfoSample {
            time_seconds: now,
            position,
            yaw: rotation.map_or(0.0, |rotation| rotation.yaw),
            pitch: rotation.map_or(0.0, |rotation| rotation.pitch),
            dimension: dimension_id,
            biome,
            sky_darken: dimension.as_ref().and_then(|dimension| dimension.sky_darken()).unwrap_or(0),
            target,
        };
        let biome_name = match biome {
            Some(id) => self.names.name(&self.biomes, id),
            None => None,
        };
        let names =
            InfoNames { biome: biome_name, target_block: self.has_target_name.then_some(self.target_name.as_str()) };
        panel.refresh(&sample, names, &self.feet_column[..count]);
        scanned
    }

    // the buffers of the reader and the names it holds
    pub fn resident_bytes(&self) -> u64 {
        (std::mem::size_of::<Self>() + self.target_name.capacity()) as u64 + self.names.resident_bytes()
    }

    // whether the game served the facts
    fn read_world(info: Option<WorldInfo>, panel: &mut InfoPanel) -> bool {
        let served = info.is_some();
        let connection = match &info {
            Some(info) if info.realm == Some(true) => InfoConnection::Realm,
            Some(info) => match info.connection.as_ref().map(|connection| connection.kind) {
                Some(ConnectionKind::Local) => InfoConnection::Local,
                Some(ConnectionKind::Remote) => InfoConnection::Server,
                _ => InfoConnection::Unknown,
            },
            None => InfoConnection::Unknown,
        };
        panel.set_world(InfoWorldFacts {
            level_name: info.as_ref().and_then(|info| info.level_name.as_deref()),
            seed: info.as_ref().and_then(|info| info.seed),
            connection,
        });
        served
    }

    fn scan_entities(&mut self, region: &BlockSource<'_>, position: Vec3f, panel: &mut InfoPanel, now: f64) {
        let (min, max) = panel.entity_scan_box(position);
        let aabb = Aabb { min: Vec3 { x: min.x, y: min.y, z: min.z }, max: Vec3 { x: max.x, y: max.y, z: max.z } };
        let count = region.actors_in_entries(aabb, &mut self.entries);
        for (sample, entry) in self.actors.iter_mut().zip(&self.entries[..count]) {
            *sample = ActorSample {
                min: Vec3f { x: entry.aabb.min.x, y: entry.aabb.min.y, z: entry.aabb.min.z },
                max: Vec3f { x: entry.aabb.max.x, y: entry.aabb.max.y, z: entry.aabb.max.z },
                actor_type: entry.actor_type.0,
                is_local_player: entry.is_local_player,
            };
        }
        panel.submit_entities(&self.actors[..count], now);
    }

    // the surfaces of the column core asks for, as core samples; the number read
    fn read_feet_column(&mut self, region: &BlockSource<'_>, request: ColumnRequest) -> usize {
        let column = ColumnPos { x: request.column.x, z: request.column.z };
        let Some(count) = region.scan_surfaces(column, request.y_min, request.y_max, &mut self.found) else {
            return 0;
        };
        let count = count.min(FEET_SURFACES);
        for (sample, found) in self.feet_column.iter_mut().zip(&self.found[..count]) {
            *sample = CoreSample {
                pos: CoreBlockPos { x: found.pos.x, y: found.pos.y, z: found.pos.z },
                block_light: found.block_light,
                sky_light: found.sky_light,
            };
        }
        count
    }

    fn read_target_name(&mut self, region: Option<&BlockSource<'_>>, pos: CoreBlockPos) {
        let at = BlockPos { x: pos.x, y: pos.y, z: pos.z };
        let name = region.and_then(|region| region.block(at)).and_then(|block| block.name());
        self.target_name.clear();
        self.has_target_name = name.is_some();
        if let Some(name) = name {
            self.target_name.push_str(&name);
        }
    }
}
