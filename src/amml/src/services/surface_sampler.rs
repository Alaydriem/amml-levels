use amml_api::{BlockSource, ColumnPos, SurfaceSample};
use levels_core::config::MAX_COLUMN_SURFACES;
use levels_core::structs::frame::ColumnRequest;
use levels_core::structs::light::SurfaceSample as CoreSample;
use levels_core::structs::world::BlockPos as CoreBlockPos;

use crate::types::SampleOutcome;

// finds the surfaces of one column with one loader call: a block whose top face a mob can stand on, with a passable
// block above, and the raw light of the block above. The loader reads the chunk and cuts the heights to it. The
// read buffer is sized once
pub struct SurfaceSampler {
    found: Vec<SurfaceSample>,
}

impl SurfaceSampler {
    pub fn new() -> Self {
        Self { found: vec![SurfaceSample::default(); MAX_COLUMN_SURFACES] }
    }

    // writes the surfaces of the requested column to `out`, lowest first, at most MAX_COLUMN_SURFACES. A column the
    // loader cannot read (the chunk is not loaded, or a block was not received yet) gives `Skipped`
    pub fn sample(
        &mut self,
        region: &BlockSource<'_>,
        request: &ColumnRequest,
        out: &mut Vec<CoreSample>,
    ) -> SampleOutcome {
        out.clear();
        let column = ColumnPos { x: request.column.x, z: request.column.z };
        let Some(count) = region.scan_surfaces(column, request.y_min, request.y_max, &mut self.found) else {
            return SampleOutcome::Skipped;
        };
        out.extend(self.found[..count.min(self.found.len())].iter().map(Self::to_core));
        SampleOutcome::Sampled(out.len())
    }

    fn to_core(sample: &SurfaceSample) -> CoreSample {
        CoreSample {
            pos: CoreBlockPos { x: sample.pos.x, y: sample.pos.y, z: sample.pos.z },
            block_light: sample.block_light,
            sky_light: sample.sky_light,
        }
    }
}

impl Default for SurfaceSampler {
    fn default() -> Self {
        Self::new()
    }
}
