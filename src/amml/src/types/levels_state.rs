use crate::services::{InfoDrawer, InfoReader, ScanDriver, WorldDrawer};
use crate::types::LinkStage;

// everything behind the lock of the runtime
pub struct LevelsState {
    pub stage: LinkStage,
    pub scan: ScanDriver,
    pub drawer: WorldDrawer,
    pub info: InfoReader,
    pub hud: InfoDrawer,
    // whether the last tick saw the info panel shown, to log a change once
    pub info_shown: bool,
    // whether the last tick saw a world loaded
    pub in_world: bool,
    // the scan line of this overlay run was written
    pub scan_logged: bool,
    // the microseconds the last scan took
    pub sample_us: f32,
    // the graphics mode of the last drawn frame, to log a change once
    pub last_vibrant: Option<bool>,
    // when the last telemetry draw line was written
    pub drawn_logged_at: Option<f64>,
}
