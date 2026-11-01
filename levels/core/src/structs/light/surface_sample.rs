use crate::structs::world::BlockPos;

// pos is the solid block; the light values are read in the block above it
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SurfaceSample {
    pub pos: BlockPos,
    pub block_light: u8,
    pub sky_light: u8,
}
