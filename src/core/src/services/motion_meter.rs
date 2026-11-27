use crate::structs::world::Vec3f;

// Mean player speed over the last `MOTION_SAMPLES` client ticks.
pub struct MotionMeter {
    last: Option<(Vec3f, i32, f64)>,
    horizontal: [f32; Self::MOTION_SAMPLES],
    total: [f32; Self::MOTION_SAMPLES],
    next: usize,
    count: usize,
}

impl MotionMeter {
    pub const MOTION_SAMPLES: usize = 10;
    pub const MAX_STEP_BLOCKS: f32 = 16.0;

    pub fn new() -> Self {
        Self {
            last: None,
            horizontal: [0.0; Self::MOTION_SAMPLES],
            total: [0.0; Self::MOTION_SAMPLES],
            next: 0,
            count: 0,
        }
    }

    pub fn reset(&mut self) {
        self.last = None;
        self.clear_steps();
    }

    pub fn sample(&mut self, pos: Vec3f, dimension: i32, time_seconds: f64) {
        let previous = self.last.replace((pos, dimension, time_seconds));
        let Some((from, from_dimension, from_time)) = previous else {
            return;
        };
        let dt = time_seconds - from_time;
        if from_dimension != dimension || dt <= 0.0 || !dt.is_finite() {
            self.clear_steps();
            return;
        }
        let dx = pos.x - from.x;
        let dy = pos.y - from.y;
        let dz = pos.z - from.z;
        let horizontal = (dx * dx + dz * dz).sqrt();
        let total = (horizontal * horizontal + dy * dy).sqrt();
        if !total.is_finite() || total > Self::MAX_STEP_BLOCKS {
            self.clear_steps();
            return;
        }
        let dt = dt as f32;
        self.horizontal[self.next] = horizontal / dt;
        self.total[self.next] = total / dt;
        self.next = (self.next + 1) % Self::MOTION_SAMPLES;
        self.count = (self.count + 1).min(Self::MOTION_SAMPLES);
    }

    // (horizontal, total) in blocks per second.
    pub fn speed(&self) -> Option<(f32, f32)> {
        if self.count == 0 {
            return None;
        }
        let n = self.count as f32;
        let horizontal: f32 = self.horizontal[..self.count].iter().sum();
        let total: f32 = self.total[..self.count].iter().sum();
        Some((horizontal / n, total / n))
    }

    fn clear_steps(&mut self) {
        self.next = 0;
        self.count = 0;
    }
}

impl Default for MotionMeter {
    fn default() -> Self {
        Self::new()
    }
}
