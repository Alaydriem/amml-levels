use crate::structs::telemetry::TelemetryReport;

const WINDOW_SECONDS: f64 = 2.0;
const MS_PER_SECOND: f64 = 1000.0;

// fixed counters over one window; take_report hands out a report once per window and starts the next
pub struct Telemetry {
    enabled: bool,
    budget_bytes: u64,
    memory_bytes: u64,
    window_start: Option<f64>,
    last_frame: Option<f64>,
    frames: u32,
    intervals: u32,
    max_frame: f64,
    columns: u64,
    labels: u32,
    culled: u32,
    draws_without_camera: u32,
    timings: u32,
    sample_us: f64,
    draw_us: f64,
    markers: u32,
    quads_drawn: u32,
    meshes_built: u32,
    world_timings: u32,
    world_build_us: f64,
    world_draw_us: f64,
}

impl Telemetry {
    pub fn new(enabled: bool, budget_bytes: u64) -> Self {
        Self {
            enabled,
            budget_bytes,
            memory_bytes: 0,
            window_start: None,
            last_frame: None,
            frames: 0,
            intervals: 0,
            max_frame: 0.0,
            columns: 0,
            labels: 0,
            culled: 0,
            draws_without_camera: 0,
            timings: 0,
            sample_us: 0.0,
            draw_us: 0.0,
            markers: 0,
            quads_drawn: 0,
            meshes_built: 0,
            world_timings: 0,
            world_build_us: 0.0,
            world_draw_us: 0.0,
        }
    }

    // a change starts a fresh window, so a report never spans the time telemetry was off
    pub fn set_enabled(&mut self, enabled: bool) {
        if enabled != self.enabled {
            *self = Self { memory_bytes: self.memory_bytes, ..Self::new(enabled, self.budget_bytes) };
        }
    }

    pub fn frame(&mut self, time_seconds: f64, columns: usize) {
        if !self.enabled {
            return;
        }
        if let Some(last) = self.last_frame {
            self.intervals += 1;
            self.max_frame = self.max_frame.max(time_seconds - last);
        }
        self.window_start.get_or_insert(time_seconds);
        self.last_frame = Some(time_seconds);
        self.frames += 1;
        self.columns += columns as u64;
    }

    pub fn labels(&mut self, count: usize, culled: usize) {
        if self.enabled {
            self.labels = u32::try_from(count).unwrap_or(u32::MAX);
            self.culled = u32::try_from(culled).unwrap_or(u32::MAX);
        }
    }

    pub fn camera_missing(&mut self) {
        if self.enabled {
            self.draws_without_camera = self.draws_without_camera.saturating_add(1);
        }
    }

    // the resident memory now, reported at the end of the window
    pub fn memory(&mut self, bytes: u64) {
        self.memory_bytes = bytes;
    }

    pub fn timing(&mut self, sample_us: f32, draw_us: f32) {
        if self.enabled {
            self.timings += 1;
            self.sample_us += sample_us as f64;
            self.draw_us += draw_us as f64;
        }
    }

    pub fn world(&mut self, markers: usize, quads: usize) {
        if self.enabled {
            self.markers = u32::try_from(markers).unwrap_or(u32::MAX);
            self.quads_drawn = u32::try_from(quads).unwrap_or(u32::MAX);
        }
    }

    pub fn world_timing(&mut self, build_us: f32, draw_us: f32, meshes_built: u32) {
        if self.enabled {
            self.world_timings += 1;
            self.world_build_us += build_us as f64;
            self.world_draw_us += draw_us as f64;
            self.meshes_built = self.meshes_built.saturating_add(meshes_built);
        }
    }

    pub fn take_report(&mut self, time_seconds: f64) -> Option<TelemetryReport> {
        let start = self.window_start.filter(|_| self.enabled)?;
        let elapsed = time_seconds - start;
        if elapsed < WINDOW_SECONDS {
            return None;
        }
        let report = TelemetryReport {
            fps: (self.intervals as f64 / elapsed) as f32,
            max_frame_ms: (self.max_frame * MS_PER_SECOND) as f32,
            labels: self.labels,
            culled: self.culled,
            draws_without_camera: self.draws_without_camera,
            columns_per_frame: Self::average(self.columns as f64, self.frames) as f32,
            sample_us_avg: Self::average(self.sample_us, self.timings) as f32,
            draw_us_avg: Self::average(self.draw_us, self.timings) as f32,
            markers: self.markers,
            quads_drawn: self.quads_drawn,
            meshes_built: self.meshes_built,
            world_build_us_avg: Self::average(self.world_build_us, self.world_timings) as f32,
            world_draw_us_avg: Self::average(self.world_draw_us, self.world_timings) as f32,
            memory_bytes: self.memory_bytes,
            budget_bytes: self.budget_bytes,
        };
        self.reset_window(time_seconds);
        Some(report)
    }

    fn reset_window(&mut self, time_seconds: f64) {
        self.window_start = Some(time_seconds);
        self.frames = 0;
        self.intervals = 0;
        self.max_frame = 0.0;
        self.columns = 0;
        self.draws_without_camera = 0;
        self.timings = 0;
        self.sample_us = 0.0;
        self.draw_us = 0.0;
        self.meshes_built = 0;
        self.world_timings = 0;
        self.world_build_us = 0.0;
        self.world_draw_us = 0.0;
    }

    fn average(sum: f64, count: u32) -> f64 {
        if count == 0 { 0.0 } else { sum / count as f64 }
    }
}
