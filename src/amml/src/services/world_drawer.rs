use amml_api::{ApiError, BlockPos, ColorQuad, MeshId, Transform, Vec3, WorldCanvas};
use levels_core::config::{MAX_LABELS, MAX_MARKERS};
use levels_core::services::{ChunkBorderPlanner, LabelMeshBuilder, SphereShellBuilder};
use levels_core::structs::marker::WorldDrawPlan;
use levels_core::structs::world::BlockPos as CoreBlockPos;

use crate::services::{FrameStatusReader, LevelsLog, MeshSlot};
use crate::types::{WorldDrawStats, WorldSlot};

/// Draws core's world plan from the level render: the light numbers once, the chunk border meshes once, then each
/// marker's shell pair (inner and outer in one mesh, so one draw per marker). Bound: three mesh slots, each holding the
/// meshes its largest quad list takes (`LABEL_MESHES`, `BORDER_MESHES`, `SHELL_MESHES`) and rebuilt only when its plan
/// version changes or the loader freed its meshes; the draw order buffer holds one entry per marker at most and the
/// quad buffer one mesh of quads, both reserved once
pub struct WorldDrawer {
    label: MeshSlot,
    border: MeshSlot,
    shell: MeshSlot,
    scratch: Vec<ColorQuad>,
    // the squared camera distance and the plan index of each marker instance
    order: Vec<(f64, usize)>,
    log: LevelsLog,
}

impl WorldDrawer {
    pub const LABEL_MESHES: usize = MeshSlot::meshes_for(LabelMeshBuilder::MAX_QUADS_PER_LABEL * MAX_LABELS as usize);
    pub const BORDER_MESHES: usize = MeshSlot::meshes_for(ChunkBorderPlanner::max_quads());
    pub const SHELL_MESHES: usize = MeshSlot::meshes_for(SphereShellBuilder::pair_quad_count());

    pub fn new(log: LevelsLog) -> Self {
        Self {
            label: MeshSlot::new(WorldSlot::Label, Self::LABEL_MESHES, log),
            border: MeshSlot::new(WorldSlot::Border, Self::BORDER_MESHES, log),
            shell: MeshSlot::new(WorldSlot::Shell, Self::SHELL_MESHES, log),
            scratch: Vec::with_capacity(MeshSlot::MAX_QUADS),
            order: Vec::with_capacity(MAX_MARKERS as usize),
            log,
        }
    }

    /// Rebuilds the meshes whose plan version changed and draws the plan: the labels, the border, then each marker's
    /// shell pair, farthest first. A `None` plan frees the meshes
    pub fn render(&mut self, canvas: &WorldCanvas<'_>, plan: Option<WorldDrawPlan<'_>>) -> WorldDrawStats {
        let mut stats = WorldDrawStats::default();
        let Some(plan) = plan else {
            self.clear(canvas);
            return stats;
        };
        let start = FrameStatusReader::now_seconds();
        self.sync(canvas, &plan, &mut stats);
        let built = FrameStatusReader::now_seconds();
        // a draw that finds a mesh freed by the loader (a graphics mode switch) leaves its slot empty: the slot is
        // built again and drawn in the same frame, so no frame goes without it
        let stale = self.draw(canvas, &plan, &mut stats);
        let mut drawn = FrameStatusReader::now_seconds();
        let mut rebuild_us = 0.0;
        if stale {
            self.sync(canvas, &plan, &mut stats);
            let rebuilt = FrameStatusReader::now_seconds();
            rebuild_us = ((rebuilt - drawn) * 1_000_000.0) as f32;
            self.draw(canvas, &plan, &mut stats);
            drawn = FrameStatusReader::now_seconds();
        }
        stats.build_us = ((built - start) * 1_000_000.0) as f32 + rebuild_us;
        stats.draw_us = (((drawn - built) * 1_000_000.0) as f32 - rebuild_us).max(0.0);
        stats
    }

    fn sync(&mut self, canvas: &WorldCanvas<'_>, plan: &WorldDrawPlan<'_>, stats: &mut WorldDrawStats) {
        self.label.sync(canvas, plan.label_version, plan.label_quads, &mut self.scratch, stats);
        self.border.sync(canvas, plan.border_version, plan.border_quads, &mut self.scratch, stats);
        self.shell.sync(canvas, plan.shell_version, plan.shell_quads, &mut self.scratch, stats);
    }

    // frees every mesh: nothing is drawn any more
    pub fn clear(&mut self, canvas: &WorldCanvas<'_>) {
        self.label.clear(canvas);
        self.border.clear(canvas);
        self.shell.clear(canvas);
    }

    // forgets every mesh id: the loader freed them at the world leave
    pub fn leave(&mut self) {
        self.label.forget();
        self.border.forget();
        self.shell.forget();
    }

    // an estimate: the buffers of the mod; the meshes are the loader's
    pub fn resident_bytes(&self) -> u64 {
        let quads = self.scratch.capacity() * size_of::<ColorQuad>();
        let order = self.order.capacity() * size_of::<(f64, usize)>();
        let ids = (Self::LABEL_MESHES + Self::BORDER_MESHES + Self::SHELL_MESHES) * size_of::<MeshId>();
        (quads + order + ids) as u64
    }

    // true when a mesh was found freed by the loader
    fn draw(&mut self, canvas: &WorldCanvas<'_>, plan: &WorldDrawPlan<'_>, stats: &mut WorldDrawStats) -> bool {
        let mut stale = Self::draw_slot(canvas, &mut self.label, Self::at_origin(plan.label_origin), self.log, stats);
        stale |= Self::draw_slot(canvas, &mut self.border, Self::at_origin(plan.border_origin), self.log, stats);
        if self.shell.meshes().is_empty() {
            return stale;
        }
        let camera = canvas.camera_position();
        self.order.clear();
        let room = self.order.capacity();
        self.order.extend(
            plan.instances
                .iter()
                .enumerate()
                .take(room)
                .map(|(index, instance)| (Self::distance_squared(instance.center, camera), index)),
        );
        // farthest first, so the nearer translucent shells blend over the farther ones
        self.order.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
        for position in 0..self.order.len() {
            let instance = plan.instances[self.order[position].1];
            let transform = Self::shell_transform(instance.center, instance.spin_degrees);
            stale |= Self::draw_slot(canvas, &mut self.shell, transform, self.log, stats);
        }
        stale
    }

    // draws each mesh of the slot with `transform`; true when a mesh was freed by the loader, which empties the slot
    fn draw_slot(
        canvas: &WorldCanvas<'_>,
        slot: &mut MeshSlot,
        transform: Transform,
        log: LevelsLog,
        stats: &mut WorldDrawStats,
    ) -> bool {
        let mut stale = false;
        let mut refused = None;
        for mesh in slot.meshes() {
            match canvas.draw(*mesh, transform) {
                Ok(()) => stats.meshes_drawn += 1,
                Err(ApiError::Stale) => stale = true,
                // the loader's white texture is not loaded yet: the next frame draws
                Err(ApiError::NotReady) => {}
                Err(error) => refused = Some(error),
            }
        }
        if let Some(error) = refused {
            log.draw_refused(slot.slot(), &error.to_string());
        }
        if stale {
            slot.mark_stale(canvas);
        }
        stale
    }

    // the mesh point (0, 0, 0) goes to the block `origin`, unturned
    fn at_origin(origin: CoreBlockPos) -> Transform {
        Transform {
            origin: BlockPos { x: origin.x, y: origin.y, z: origin.z },
            offset: Vec3::default(),
            scale: 1.0,
            yaw_degrees: 0.0,
        }
    }

    // the block the marker is in as the origin and the rest as the offset, so a marker far from the world origin keeps
    // its sub-block position. The loader turns +Z to -X at yaw 90 and the spin turned +Z to +X, so the yaw is negated
    fn shell_transform(center: [f32; 3], spin_degrees: f32) -> Transform {
        let block = center.map(f32::floor);
        Transform {
            origin: BlockPos { x: block[0] as i32, y: block[1] as i32, z: block[2] as i32 },
            offset: Vec3 { x: center[0] - block[0], y: center[1] - block[1], z: center[2] - block[2] },
            scale: 1.0,
            yaw_degrees: -spin_degrees,
        }
    }

    // without a camera position the markers keep the order of the plan
    fn distance_squared(center: [f32; 3], camera: Option<Vec3>) -> f64 {
        let Some(camera) = camera else { return 0.0 };
        let camera = [camera.x, camera.y, camera.z];
        (0..3).map(|axis| (f64::from(center[axis]) - f64::from(camera[axis])).powi(2)).sum()
    }
}
