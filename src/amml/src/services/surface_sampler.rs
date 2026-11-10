use amml_game::{Block, BlockPos, BlockSource, ChunkBlockPos, LevelChunk};
use levels_core::config::MAX_COLUMN_SURFACES;
use levels_core::structs::frame::ColumnRequest;
use levels_core::structs::light::SurfaceSample;
use levels_core::structs::world::BlockPos as CoreBlockPos;

use crate::types::SampleOutcome;

// finds the surfaces of one column: a block whose top face a mob can stand on, with a passable block above, and the
// light of the block above
pub struct SurfaceSampler;

impl SurfaceSampler {
    // the top face
    const FACE_UP: u8 = 1;

    // writes the surfaces of the requested column to `out`, lowest first, at most MAX_COLUMN_SURFACES. The chunk is
    // looked up once. A chunk that is not loaded gives `Skipped` and no read. The bounds of the chunk are respected:
    // no block is read below its minimum height, and none at or above its maximum height, so the block above the
    // highest surface is still inside the chunk
    pub fn sample(region: &BlockSource<'_>, request: &ColumnRequest, out: &mut Vec<SurfaceSample>) -> SampleOutcome {
        out.clear();
        let (x, z) = (request.column.x, request.column.z);
        let Some(chunk) = region.chunk(x >> 4, z >> 4) else { return SampleOutcome::Skipped };
        if !chunk.loaded() {
            return SampleOutcome::Skipped;
        }
        let (Some(min_y), Some(max_y)) = (chunk.min_y(), chunk.max_y()) else { return SampleOutcome::Skipped };
        let low = request.y_min.max(min_y);
        let high = request.y_max.min(max_y.saturating_sub(2));
        if low > high {
            return SampleOutcome::Sampled(0);
        }
        let Some(mut block) = Self::block_at(&chunk, x, low, z, min_y) else { return SampleOutcome::Sampled(0) };
        for y in low..=high {
            if out.len() >= MAX_COLUMN_SURFACES {
                break;
            }
            let Some(above) = Self::block_at(&chunk, x, y + 1, z, min_y) else { break };
            let above_pos = BlockPos { x, y: y + 1, z };
            if Self::passable(region, &above, &above_pos)
                && block.can_provide_full_support(Self::FACE_UP) == Some(true)
                && let Some(light) = region.brightness(&above_pos)
            {
                out.push(SurfaceSample {
                    pos: CoreBlockPos { x, y, z },
                    block_light: light.block,
                    sky_light: light.sky,
                });
            }
            block = above;
        }
        SampleOutcome::Sampled(out.len())
    }

    // the block at world (x, y, z) through the chunk of its column
    fn block_at<'a>(chunk: &LevelChunk<'a>, x: i32, y: i32, z: i32, min_y: i32) -> Option<Block<'a>> {
        let relative = i16::try_from(y.checked_sub(min_y)?).ok()?;
        chunk.block(ChunkBlockPos { x: (x & 15) as u8, z: (z & 15) as u8, y: relative })
    }

    // air, or a block whose collision shape is empty: flowers, torches, short grass
    fn passable(region: &BlockSource<'_>, block: &Block<'_>, pos: &BlockPos) -> bool {
        if block.is_air() == Some(true) {
            return true;
        }
        block.collision_shape(region, pos).is_some_and(|shape| shape.is_empty())
    }
}
