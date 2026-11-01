// one telemetry window: frame rate and cost of the overlay, averaged over the window, and the resident
// memory at its end against the configured budget
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TelemetryReport {
    pub fps: f32,
    pub max_frame_ms: f32,
    pub labels: u32,
    // labels left out behind the camera in the last draw of the window
    pub culled: u32,
    // draws in the window with the overlay on and no camera direction, so nothing was culled
    pub draws_without_camera: u32,
    pub columns_per_frame: f32,
    pub sample_us_avg: f32,
    pub draw_us_avg: f32,
    // spawn spheres drawn and quads submitted for the world draw in the last frame of the window
    pub markers: u32,
    pub quads_drawn: u32,
    // border or shell meshes rebuilt in the window
    pub meshes_built: u32,
    pub world_build_us_avg: f32,
    pub world_draw_us_avg: f32,
    pub memory_bytes: u64,
    pub budget_bytes: u64,
}

impl TelemetryReport {
    pub fn is_over_budget(&self) -> bool {
        self.memory_bytes > self.budget_bytes
    }
}
