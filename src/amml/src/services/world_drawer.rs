use levels_core::config::{MAX_LABELS, MAX_MARKERS};
use levels_core::services::{ChunkBorderPlanner, LabelMeshBuilder, SphereShellBuilder};
use levels_core::structs::marker::WorldDrawPlan;

use crate::services::{FrameStatusReader, MeshFactory, MeshSlot};
use crate::types::{WorldDrawStats, WorldFrame, WorldSlot};

/// Draws core's world plan from the level render: the light numbers once, the chunk border meshes once, then each
/// marker's shell pair (inner and outer in one mesh, so one draw per marker). Bound: three mesh slots, each holding the meshes its largest
/// quad list takes (`LABEL_MESHES`, `BORDER_MESHES`, `SHELL_MESHES`) and rebuilt only when its plan version changes; the draw order
/// buffer holds one entry per marker at most and is reserved once. The meshes and the factory's texture belong to the render device: they are
/// dropped on the render thread only, by `leave`
pub struct WorldDrawer {
    factory: MeshFactory,
    label: MeshSlot,
    border: MeshSlot,
    shell: MeshSlot,
    // the squared camera distance and the plan index of each marker instance
    order: Vec<(f64, usize)>,
}

impl WorldDrawer {
    pub const LABEL_MESHES: usize = MeshSlot::meshes_for(LabelMeshBuilder::MAX_QUADS_PER_LABEL * MAX_LABELS as usize);
    pub const BORDER_MESHES: usize = MeshSlot::meshes_for(ChunkBorderPlanner::max_quads());
    pub const SHELL_MESHES: usize = MeshSlot::meshes_for(SphereShellBuilder::pair_quad_count());

    pub fn new(factory: MeshFactory) -> Self {
        Self {
            factory,
            label: MeshSlot::new(WorldSlot::Label, Self::LABEL_MESHES),
            border: MeshSlot::new(WorldSlot::Border, Self::BORDER_MESHES),
            shell: MeshSlot::new(WorldSlot::Shell, Self::SHELL_MESHES),
            order: Vec::with_capacity(MAX_MARKERS as usize),
        }
    }

    pub fn factory(&self) -> &MeshFactory {
        &self.factory
    }

    /// Rebuilds the meshes whose plan version changed and draws the plan: the labels, the border, then each marker's shell
    /// pair, farthest first. A `None` plan draws nothing and keeps the meshes
    ///
    /// # Safety
    /// `frame` must hold the live screen, client and material of the running render call. Render thread only
    #[allow(unsafe_code)]
    pub unsafe fn render(&mut self, frame: &WorldFrame<'_>, plan: Option<WorldDrawPlan<'_>>) -> WorldDrawStats {
        let mut stats = WorldDrawStats::default();
        let Some(plan) = plan else { return stats };
        // SAFETY: forwarded to the caller
        if !unsafe { self.factory.prepare(frame) } {
            return stats;
        }
        let start = FrameStatusReader::now_seconds();
        // SAFETY: forwarded to the caller
        unsafe { self.rebuild(frame, &plan, &mut stats) };
        let built = FrameStatusReader::now_seconds();
        // SAFETY: forwarded to the caller
        unsafe { self.draw(frame, &plan) };
        let drawn = FrameStatusReader::now_seconds();
        stats.build_us = ((built - start) * 1_000_000.0) as f32;
        stats.draw_us = ((drawn - built) * 1_000_000.0) as f32;
        stats
    }

    /// sets the sides of the quads: two-sided for a material that culls back faces. A change drops every mesh, so
    /// each slot is built again with the new sides. Render thread only
    pub fn set_two_sided(&mut self, two_sided: bool) {
        if self.factory.set_two_sided(two_sided) {
            self.label.reset();
            self.border.reset();
            self.shell.reset();
        }
    }

    // drops every mesh, then the factory's texture. World leave, region change and an enable's first frame
    pub fn leave(&mut self) {
        self.label.reset();
        self.border.reset();
        self.shell.reset();
        self.factory.release();
    }

    // an estimate: the mesh mirrors and their vertex data; not the white texture, its location or the GPU buffers
    pub fn resident_bytes(&self) -> u64 {
        let meshes = self.label.resident_bytes() + self.border.resident_bytes() + self.shell.resident_bytes();
        if meshes == 0 {
            return 0;
        }
        meshes + (self.order.capacity() * size_of::<(f64, usize)>()) as u64
    }

    #[allow(unsafe_code)]
    unsafe fn rebuild(&mut self, frame: &WorldFrame<'_>, plan: &WorldDrawPlan<'_>, stats: &mut WorldDrawStats) {
        let Self { factory, label, border, shell, .. } = self;
        // SAFETY: forwarded to the caller
        unsafe {
            label.sync(factory, frame, plan.label_version, plan.label_quads, stats);
            border.sync(factory, frame, plan.border_version, plan.border_quads, stats);
            shell.sync(factory, frame, plan.shell_version, plan.shell_quads, stats);
        }
    }

    #[allow(unsafe_code)]
    unsafe fn draw(&mut self, frame: &WorldFrame<'_>, plan: &WorldDrawPlan<'_>) {
        let Self { factory, label, border, shell, order } = self;
        let origin = plan.label_origin;
        let label_offset = Self::offset([origin.x as f64, origin.y as f64, origin.z as f64], frame.camera);
        for mesh in label.meshes() {
            // SAFETY: forwarded to the caller
            unsafe { factory.draw(frame, WorldSlot::Label, mesh, label_offset, 0.0) };
        }
        let origin = plan.border_origin;
        let border_offset = Self::offset([origin.x as f64, origin.y as f64, origin.z as f64], frame.camera);
        for mesh in border.meshes() {
            // SAFETY: forwarded to the caller
            unsafe { factory.draw(frame, WorldSlot::Border, mesh, border_offset, 0.0) };
        }
        let meshes = shell.meshes();
        if meshes.is_empty() {
            return;
        }
        order.clear();
        let room = order.capacity();
        order.extend(
            plan.instances
                .iter()
                .enumerate()
                .take(room)
                .map(|(index, instance)| (Self::distance_squared(instance.center, frame.camera), index)),
        );
        order.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
        for &(_, index) in order.iter() {
            let instance = plan.instances[index];
            let offset = Self::offset(instance.center.map(f64::from), frame.camera);
            for mesh in meshes {
                // SAFETY: forwarded to the caller
                unsafe { factory.draw(frame, WorldSlot::Shell, mesh, offset, instance.spin_degrees) };
            }
        }
    }

    // `point - camera`, subtracted in f64 and only then cast: f32 near x = 30000 has about 2 mm resolution
    fn offset(point: [f64; 3], camera: [f32; 3]) -> [f32; 3] {
        std::array::from_fn(|axis| (point[axis] - f64::from(camera[axis])) as f32)
    }

    fn distance_squared(center: [f32; 3], camera: [f32; 3]) -> f64 {
        (0..3).map(|axis| (f64::from(center[axis]) - f64::from(camera[axis])).powi(2)).sum()
    }
}
