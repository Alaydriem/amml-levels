use crate::config::ScanConfig;
use crate::structs::frame::ColumnRequest;
use crate::structs::world::{BlockPos, ColumnPos};

// hands out a bounded slice of the column disk around the player each frame, so one full pass takes pass_seconds
pub struct ScanPlanner {
    radius: i32,
    pass_seconds: f32,
    max_columns_per_frame: usize,
    // disk offsets from the feet column, nearest first; built once
    offsets: Vec<ColumnPos>,
    cursor: usize,
    // reused every frame, capacity max_columns_per_frame
    requests: Vec<ColumnRequest>,
}

impl ScanPlanner {
    pub fn new(config: &ScanConfig) -> Self {
        let radius = config.radius as i32;
        let max_columns_per_frame = config.max_columns_per_frame.max(1) as usize;
        Self {
            radius,
            pass_seconds: config.pass_seconds,
            max_columns_per_frame,
            offsets: Self::disk(radius),
            cursor: 0,
            requests: Vec::with_capacity(max_columns_per_frame),
        }
    }

    pub fn total_columns(&self) -> usize {
        self.offsets.len()
    }

    pub fn resident_bytes(&self) -> u64 {
        (self.offsets.capacity() * std::mem::size_of::<ColumnPos>()
            + self.requests.capacity() * std::mem::size_of::<ColumnRequest>()) as u64
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
        self.requests.clear();
    }

    pub fn plan(&mut self, feet: BlockPos, dt: f32) -> &[ColumnRequest] {
        let total = self.offsets.len();
        let count = self.count_for(dt).min(total);
        self.requests.clear();
        for _ in 0..count {
            let offset = self.offsets[self.cursor];
            self.cursor = (self.cursor + 1) % total;
            self.requests.push(ColumnRequest {
                column: ColumnPos { x: feet.x + offset.x, z: feet.z + offset.z },
                y_min: feet.y - self.radius,
                y_max: feet.y + self.radius,
            });
        }
        &self.requests
    }

    fn count_for(&self, dt: f32) -> usize {
        let wanted = (self.offsets.len() as f32 * dt / self.pass_seconds).ceil();
        let wanted = if wanted.is_finite() { wanted as usize } else { self.max_columns_per_frame };
        wanted.clamp(1, self.max_columns_per_frame)
    }

    fn disk(radius: i32) -> Vec<ColumnPos> {
        let mut offsets = Vec::new();
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if dx * dx + dz * dz <= radius * radius {
                    offsets.push(ColumnPos { x: dx, z: dz });
                }
            }
        }
        offsets.sort_by_key(|o| o.x * o.x + o.z * o.z);
        offsets
    }
}
