use amml_game::ClientInstance;
use levels_core::config::MAX_COLUMN_SURFACES;
use levels_core::structs::light::SurfaceSample;

use crate::services::{FrameStatusReader, Overlay, SurfaceSampler};
use crate::types::SampleOutcome;

// the per-tick scan: reads the frame status, takes the columns core asks for, samples each and submits what it found.
// The sample buffer is sized once
pub struct ScanDriver {
    samples: Vec<SurfaceSample>,
}

impl ScanDriver {
    pub fn new() -> Self {
        Self { samples: Vec::with_capacity(MAX_COLUMN_SURFACES) }
    }

    // one tick; the microseconds the sampling took
    pub fn tick(&mut self, client: &ClientInstance<'_>, overlay: &mut Overlay) -> f32 {
        let status = FrameStatusReader::read(client, FrameStatusReader::now_seconds());
        let count = overlay.frame(status);
        if count == 0 {
            return 0.0;
        }
        let Some(region) = client.local_player().and_then(|player| player.block_source()) else { return 0.0 };
        let start = FrameStatusReader::now_seconds();
        for index in 0..count {
            let Some(request) = overlay.column(index) else { break };
            if let SampleOutcome::Sampled(_) = SurfaceSampler::sample(&region, &request, &mut self.samples) {
                overlay.submit_column(request.column, &self.samples);
            }
        }
        ((FrameStatusReader::now_seconds() - start) * 1_000_000.0) as f32
    }
}

impl Default for ScanDriver {
    fn default() -> Self {
        Self::new()
    }
}
