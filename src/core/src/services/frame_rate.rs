// Frames per second and levels frame cost over one-second windows.
pub struct FrameRate {
    window: Option<(u64, f64)>,
    cost_sum: f32,
    cost_count: u32,
    fps: Option<f32>,
    cost_us: Option<f32>,
}

impl FrameRate {
    pub const WINDOW_SECONDS: f64 = 1.0;

    pub fn new() -> Self {
        Self { window: None, cost_sum: 0.0, cost_count: 0, fps: None, cost_us: None }
    }

    pub fn sample(&mut self, total_frames: u64, time_seconds: f64) {
        let Some((start_frames, start_time)) = self.window else {
            self.window = Some((total_frames, time_seconds));
            return;
        };
        let elapsed = time_seconds - start_time;
        if elapsed < Self::WINDOW_SECONDS {
            return;
        }
        let frames = total_frames.saturating_sub(start_frames);
        self.fps = Some((frames as f64 / elapsed) as f32);
        self.cost_us = if self.cost_count > 0 { Some(self.cost_sum / self.cost_count as f32) } else { None };
        self.cost_sum = 0.0;
        self.cost_count = 0;
        self.window = Some((total_frames, time_seconds));
    }

    pub fn cost(&mut self, frame_us: f32) {
        self.cost_sum += frame_us;
        self.cost_count = self.cost_count.saturating_add(1);
    }

    pub fn fps(&self) -> Option<f32> {
        self.fps
    }

    pub fn frame_ms(&self) -> Option<f32> {
        self.fps.filter(|fps| *fps > 0.0).map(|fps| 1000.0 / fps)
    }

    pub fn cost_us(&self) -> Option<f32> {
        self.cost_us
    }
}

impl Default for FrameRate {
    fn default() -> Self {
        Self::new()
    }
}
