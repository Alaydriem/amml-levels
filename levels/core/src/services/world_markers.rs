use crate::config::{BordersConfig, SpheresConfig};
use crate::services::{ChunkBorderPlanner, MarkerRegistry, SphereShellBuilder};
use crate::structs::draw::ColorQuad;
use crate::structs::frame::FrameStatus;
use crate::structs::manager::ManagerSignal;
use crate::structs::marker::{BorderMode, MarkerInstance, WorldDrawPlan};
use crate::structs::world::BlockPos;

const FULL_TURN: f64 = 360.0;

// The chunk borders and the spawn spheres: their state, their lease hiding and the per-frame draw plan.
// The marker count is bounded by max_markers at construction; the instance buffer holds one instance per marker and is reserved once.
pub struct WorldMarkers {
    spheres: SpheresConfig,
    planner: ChunkBorderPlanner,
    registry: MarkerRegistry,
    builder: SphereShellBuilder,
    shell_quads: Vec<ColorQuad>,
    shell_version: u64,
    instances: Vec<MarkerInstance>,
    is_lease_hidden: bool,
    is_visible: bool,
}

impl WorldMarkers {
    pub fn new(borders: BordersConfig, spheres: SpheresConfig) -> Self {
        let max = spheres.max_markers as usize;
        let mut markers = Self {
            planner: ChunkBorderPlanner::new(borders),
            registry: MarkerRegistry::new(max),
            builder: SphereShellBuilder::new(),
            shell_quads: Vec::with_capacity(SphereShellBuilder::pair_quad_count()),
            shell_version: 0,
            instances: Vec::with_capacity(max),
            is_lease_hidden: false,
            is_visible: false,
            spheres,
        };
        markers.rebuild_shells();
        markers
    }

    pub fn signal(&mut self, signal: ManagerSignal, status: Option<FrameStatus>) {
        match signal {
            ManagerSignal::BordersPressed => {
                if Self::accepts_input(status) {
                    self.planner.cycle();
                }
            }
            ManagerSignal::SphereDropPressed => {
                if let Some(status) = status.filter(|s| Self::accepts_input(Some(*s))) {
                    self.registry.drop_at(status.feet, status.dimension);
                }
            }
            ManagerSignal::SphereRemovePressed => {
                if let Some(status) = status.filter(|s| Self::accepts_input(Some(*s))) {
                    self.registry.remove_nearest(Self::player(&status), status.dimension);
                }
            }
            ManagerSignal::SphereClearPressed => {
                if Self::accepts_input(status) {
                    self.registry.clear();
                }
            }
            ManagerSignal::LeaseLost | ManagerSignal::LeaseSuspended => self.is_lease_hidden = true,
            ManagerSignal::LeaseRestored => self.is_lease_hidden = false,
            ManagerSignal::Disabled => self.clear_all(),
            ManagerSignal::TogglePressed => {}
        }
    }

    pub fn frame(&mut self, status: &FrameStatus) {
        if !status.in_world {
            self.is_visible = false;
            if self.is_active() || self.is_lease_hidden {
                self.clear_all();
            }
            return;
        }
        self.is_visible = status.has_player && !status.menu_open && !status.hud_hidden;
        if !self.is_visible {
            return;
        }
        self.planner.update(status.feet, status.dimension);
        self.instances.clear();
        let player = Self::player(status);
        let degrees = Self::spin(status.time_seconds, self.spheres.spin);
        for marker in self.registry.visible(player, status.dimension, self.spheres.show_distance) {
            self.instances.push(MarkerInstance { center: marker.center, spin_degrees: degrees });
        }
    }

    // `None` also when no border or marker is active, so nothing is built for a player who uses neither
    pub fn plan(&self) -> Option<WorldDrawPlan<'_>> {
        if self.is_lease_hidden || !self.is_visible || !self.is_active() {
            return None;
        }
        Some(WorldDrawPlan {
            label_origin: BlockPos::default(),
            label_version: 0,
            label_quads: &[],
            border_origin: self.planner.origin(),
            border_version: self.planner.version(),
            border_quads: self.planner.quads(),
            shell_version: self.shell_version,
            shell_quads: &self.shell_quads,
            instances: &self.instances,
        })
    }

    // the marker capacity is fixed at construction; every other sphere setting takes effect at once
    pub fn apply_config(&mut self, borders: BordersConfig, spheres: SpheresConfig) {
        self.planner.restyle(borders);
        let is_shape_changed = self.spheres.inner_radius != spheres.inner_radius
            || self.spheres.outer_radius != spheres.outer_radius
            || self.spheres.inner_color != spheres.inner_color
            || self.spheres.outer_color != spheres.outer_color;
        self.spheres = spheres;
        if is_shape_changed {
            self.rebuild_shells();
        }
    }

    // the graphics mode: under Vibrant Visuals the shells lose their translucent fill and the border lines get
    // thinner and fainter. A change rebuilds what it touches
    pub fn set_vibrant(&mut self, vibrant: bool) {
        if self.builder.set_fill(!vibrant) {
            self.rebuild_shells();
        }
        self.planner.set_vibrant(vibrant);
    }

    pub fn is_active(&self) -> bool {
        self.planner.mode() != BorderMode::Off || !self.registry.is_empty()
    }

    pub fn resident_bytes(&self) -> u64 {
        let quad = std::mem::size_of::<ColorQuad>();
        let shells = self.shell_quads.capacity() * quad;
        let instances = self.instances.capacity() * std::mem::size_of::<MarkerInstance>();
        self.planner.resident_bytes() + self.registry.resident_bytes() + (shells + instances) as u64
    }

    fn accepts_input(status: Option<FrameStatus>) -> bool {
        status.is_some_and(|s| s.in_world && !s.menu_open)
    }

    fn player(status: &FrameStatus) -> [f32; 3] {
        [status.feet.x as f32 + 0.5, status.feet.y as f32, status.feet.z as f32 + 0.5]
    }

    fn spin(time_seconds: f64, degrees_per_second: f32) -> f32 {
        ((time_seconds * degrees_per_second as f64) % FULL_TURN) as f32
    }

    fn clear_all(&mut self) {
        self.planner.reset();
        self.registry.clear();
        self.instances.clear();
        self.is_lease_hidden = false;
    }

    fn rebuild_shells(&mut self) {
        self.builder.build(self.spheres.inner_radius, self.spheres.inner_color, &mut self.shell_quads);
        self.builder.append(self.spheres.outer_radius, self.spheres.outer_color, &mut self.shell_quads);
        self.shell_version += 1;
    }
}
