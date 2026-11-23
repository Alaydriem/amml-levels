use crate::structs::world::BlockPos;

// Everything the label list and mesh depend on. Equal stamps give the same labels and the same quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelStamp {
    pub grid_revision: u64,
    pub style_revision: u64,
    pub feet: BlockPos,
    pub sky_darken: u8,
    pub quarter: u8,
}
