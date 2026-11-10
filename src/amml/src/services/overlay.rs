use std::path::Path;

use amm_client::link::ApiTable;
use amm_core::errors::LinkError;
use levels_core::services::LevelsController;
use levels_core::structs::frame::{ColumnRequest, FrameStatus};
use levels_core::structs::light::SurfaceSample;
use levels_core::structs::marker::WorldDrawPlan;
use levels_core::structs::world::ColumnPos;

use crate::services::ManagerSession;

// the overlay: core and its manager registration. The manager's events go first in each frame, so a toggle or a lost
// lease shapes the frame; the lease follows the overlay before and after the frame, since leaving the world turns the
// overlay off inside it. The buffer of column requests is sized once
pub struct Overlay {
    controller: LevelsController,
    session: ManagerSession,
    columns: Vec<ColumnRequest>,
}

impl Overlay {
    // registers with the manager and starts core from the manager's config; an unreadable config gives the defaults
    pub fn connect(table: ApiTable, config_dir: &Path) -> Result<Self, LinkError> {
        let session = ManagerSession::connect(table, config_dir)?;
        let controller = LevelsController::new(session.config().unwrap_or_default());
        Ok(Self::new(controller, session))
    }

    pub fn new(controller: LevelsController, session: ManagerSession) -> Self {
        let columns = Vec::with_capacity(controller.max_columns_per_frame());
        Self { controller, session, columns }
    }

    // takes this frame's status and keeps the columns to sample; the number of columns
    pub fn frame(&mut self, status: FrameStatus) -> usize {
        self.session.drain(&mut self.controller);
        self.session.sync_lease(&mut self.controller);
        let requests = self.controller.frame(status);
        self.columns.clear();
        let room = self.columns.capacity();
        self.columns.extend(requests.iter().take(room));
        self.session.sync_lease(&mut self.controller);
        self.columns.len()
    }

    pub fn column(&self, index: usize) -> Option<ColumnRequest> {
        self.columns.get(index).copied()
    }

    pub fn submit_column(&mut self, column: ColumnPos, samples: &[SurfaceSample]) {
        self.controller.submit_column(column, samples);
    }

    // `outside_bytes` is the memory the mod holds for the overlay beside core's own
    pub fn timing(&mut self, sample_us: f32, draw_us: f32, outside_bytes: u64) {
        let own = (self.columns.capacity() * std::mem::size_of::<ColumnRequest>()) as u64;
        self.controller.timing(sample_us, draw_us, outside_bytes.saturating_add(own));
    }

    // the world draw of this frame, light numbers included; None when nothing is to be drawn
    pub fn set_vibrant(&mut self, vibrant: bool) {
        self.controller.set_vibrant(vibrant);
    }

    pub fn world_plan(&mut self) -> Option<WorldDrawPlan<'_>> {
        self.controller.world_plan()
    }

    pub fn world_timing(&mut self, build_us: f32, draw_us: f32, meshes_built: u32) {
        self.controller.world_timing(build_us, draw_us, meshes_built);
    }

    pub fn is_in_world(&self) -> bool {
        self.controller.is_in_world()
    }

    pub fn is_on(&self) -> bool {
        self.controller.is_on()
    }

    pub fn controller(&self) -> &LevelsController {
        &self.controller
    }
}
