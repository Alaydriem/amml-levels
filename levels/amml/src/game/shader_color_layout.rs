use std::mem::offset_of;

/// Layout mirror of the game's `ShaderColor`: the colour the current material multiplies its output by, and the flag
/// that tells the renderer to upload it again. `MeshContext.currentShaderColor` points at one shared instance
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShaderColorLayout {
    pub color: [f32; 4], // 0x00 color
    pub dirty: bool,     // 0x10 dirty
}

const _: () = assert!(offset_of!(ShaderColorLayout, dirty) == 0x10);
