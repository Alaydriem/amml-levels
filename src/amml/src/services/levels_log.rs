use amml_api::Logger;

use crate::types::WorldSlot;

// the proof lines of the mod: each message starts with `levels: ` and the changing parts are fields
#[derive(Clone, Copy)]
pub struct LevelsLog {
    log: Logger,
}

impl LevelsLog {
    pub fn new(log: Logger) -> Self {
        Self { log }
    }

    pub fn manager_linked(&self) {
        self.log.info("levels: manager linked", &[]);
    }

    pub fn manager_missing(&self) {
        self.log.error("levels: manager missing, overlay off", &[]);
    }

    pub fn dimension_unreadable(&self) {
        self.log.warn("levels: dimension unreadable", &[]);
    }

    pub fn graphics_mode(&self, vibrant: bool) {
        self.log.info("levels: graphics mode seen", &[("vibrant", if vibrant { "1" } else { "0" })]);
    }

    pub fn lease_held(&self) {
        self.log.info("levels: world draw lease held", &[]);
    }

    pub fn lease_released(&self) {
        self.log.info("levels: world draw lease released", &[]);
    }

    pub fn scan(&self, columns: usize, surfaces: usize) {
        self.log.info("levels: scan", &[("columns", &columns.to_string()), ("surfaces", &surfaces.to_string())]);
    }

    pub fn mesh_built(&self, slot: WorldSlot, quads: usize) {
        self.log.info("levels: mesh built", &[("slot", slot.as_str()), ("quads", &quads.to_string())]);
    }

    pub fn meshes_stale(&self, slot: WorldSlot) {
        self.log.info("levels: meshes stale, rebuilt", &[("slot", slot.as_str())]);
    }

    pub fn build_failed(&self, slot: WorldSlot, error: &str, tries: u32) {
        self.log.warn(
            "levels: mesh build failed",
            &[("slot", slot.as_str()), ("error", error), ("tries", &tries.to_string())],
        );
    }

    pub fn build_given_up(&self, slot: WorldSlot) {
        self.log.error(
            "levels: mesh build failed again, slot stays empty until its quads change",
            &[("slot", slot.as_str())],
        );
    }

    pub fn draw_refused(&self, slot: WorldSlot, error: &str) {
        self.log.warn("levels: mesh draw refused", &[("slot", slot.as_str()), ("error", error)]);
    }

    pub fn world_drawn(&self, meshes: u32, vibrant: bool) {
        self.log.info(
            "levels: world drawn",
            &[("meshes", &meshes.to_string()), ("vibrant", if vibrant { "1" } else { "0" })],
        );
    }

    pub fn sphere_dropped(&self, count: usize) {
        self.log.info("levels: sphere dropped", &[("count", &count.to_string())]);
    }

    pub fn sphere_removed(&self, count: usize) {
        self.log.info("levels: sphere removed", &[("count", &count.to_string())]);
    }

    pub fn spheres_cleared(&self) {
        self.log.info("levels: spheres cleared", &[]);
    }
}
