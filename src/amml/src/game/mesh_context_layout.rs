use std::ffi::c_void;
use std::mem::offset_of;

use crate::game::ShaderColorLayout;

/// Layout mirror of the start of the game's `mce::MeshContext` (no vtable), which sits `ScreenContext.meshContext`
/// into a `ScreenContext`: the render context, the camera, the global constant buffers, their manager and the
/// current shader colour, each a reference stored as a pointer. A prefix: the mod only reads through the game's
/// pointer. `ScreenContext.camera`, `ScreenContext.constantBuffers` and `ScreenContext.currentShaderColor` check
/// the second, third and fifth members; the fourth is not read or written.
#[repr(C)]
pub struct MeshContextLayout {
    pub render_context: *mut c_void,                  // 0x00 renderContext
    pub camera: *mut c_void,                          // 0x08 camera
    pub constant_buffers: *mut c_void,                // 0x10 constantBuffers
    pub constant_buffer_manager: *mut c_void,         // 0x18 constantBufferManager
    pub current_shader_color: *mut ShaderColorLayout, // 0x20 currentShaderColor
}

const _: () = assert!(offset_of!(MeshContextLayout, camera) == 0x08);
const _: () = assert!(offset_of!(MeshContextLayout, constant_buffers) == 0x10);
const _: () = assert!(offset_of!(MeshContextLayout, current_shader_color) == 0x20);
