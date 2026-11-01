use crate::structs::draw::ColorQuad;
use crate::structs::marker::MarkerInstance;
use crate::structs::world::BlockPos;

// Everything the world draw needs for one frame. A version changes when its quads change, so the host
// rebuilds a mesh only then.
#[derive(Debug, Clone, Copy)]
pub struct WorldDrawPlan<'a> {
    pub border_origin: BlockPos,
    pub border_version: u64,
    pub border_quads: &'a [ColorQuad],
    pub shell_version: u64,
    // the inner shell quads first, then the outer shell quads
    pub shell_quads: &'a [ColorQuad],
    pub instances: &'a [MarkerInstance],
}
