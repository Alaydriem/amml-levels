use amml_api::Client;
use levels_core::config::MAX_COLUMN_SURFACES;
use levels_core::structs::light::SurfaceSample;

use crate::services::{FrameStatusReader, LevelsLog, Overlay, SurfaceSampler};
use crate::types::{SampleOutcome, ScanReport};

// the per-tick scan: reads the frame status, takes the columns core asks for, samples each and submits what it found.
// The sample buffers are sized once
pub struct ScanDriver {
    sampler: SurfaceSampler,
    samples: Vec<SurfaceSample>,
    // the last tick with a player could not read its dimension; logged once per change of state
    dimension_unreadable: bool,
    log: LevelsLog,
}

impl ScanDriver {
    pub fn new(log: LevelsLog) -> Self {
        Self {
            sampler: SurfaceSampler::new(),
            samples: Vec::with_capacity(MAX_COLUMN_SURFACES),
            dimension_unreadable: false,
            log,
        }
    }

    // one tick
    pub fn tick(&mut self, client: &Client<'_>, overlay: &mut Overlay) -> ScanReport {
        let (status, readable) = FrameStatusReader::read_checked(client, FrameStatusReader::now_seconds());
        self.note_dimension(readable);
        let count = overlay.frame(status);
        if count == 0 {
            return ScanReport::default();
        }
        let Some(region) = client.local_player().and_then(|player| player.block_source()) else {
            return ScanReport::default();
        };
        let start = FrameStatusReader::now_seconds();
        let (mut columns, mut surfaces) = (0, 0);
        for index in 0..count {
            let Some(request) = overlay.column(index) else { break };
            if let SampleOutcome::Sampled(found) = self.sampler.sample(&region, &request, &mut self.samples) {
                overlay.submit_column(request.column, &self.samples);
                columns += 1;
                surfaces += found;
            }
        }
        let sample_us = ((FrameStatusReader::now_seconds() - start) * 1_000_000.0) as f32;
        ScanReport { sample_us, columns, surfaces }
    }

    // one line when the dimension turns unreadable, and again only after it was readable in between
    fn note_dimension(&mut self, readable: Option<bool>) {
        match readable {
            Some(false) if !self.dimension_unreadable => {
                self.dimension_unreadable = true;
                self.log.dimension_unreadable();
            }
            Some(true) => self.dimension_unreadable = false,
            _ => {}
        }
    }
}
