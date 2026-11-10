use crate::structs::draw::ColorQuad;
use crate::structs::marker::MarkerInstance;
use crate::structs::world::BlockPos;

// Everything the world draw needs for one frame. A version changes when its quads change, so the host
// rebuilds a mesh only then. An empty quad slice with version 0 means nothing to draw.
#[derive(Debug, Clone, Copy)]
pub struct WorldDrawPlan<'a> {
    pub label_origin: BlockPos,
    pub label_version: u64,
    pub label_quads: &'a [ColorQuad],
    pub border_origin: BlockPos,
    pub border_version: u64,
    pub border_quads: &'a [ColorQuad],
    pub shell_version: u64,
    // the inner shell quads first, then the outer shell quads
    pub shell_quads: &'a [ColorQuad],
    pub instances: &'a [MarkerInstance],
}
