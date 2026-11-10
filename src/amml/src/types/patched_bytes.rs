use std::ptr::NonNull;

// the blend, blend factor and depth-write bytes of a patched `RenderMaterial`, as the game had them
#[derive(Clone, Copy)]
pub(crate) struct PatchedBytes {
    pub(crate) material: NonNull<u8>,
    pub(crate) blend: u8,
    pub(crate) source: u8,
    pub(crate) destination: u8,
    pub(crate) depth_write: u8,
}
