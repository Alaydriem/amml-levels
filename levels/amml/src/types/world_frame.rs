use std::ffi::c_void;

use crate::types::MeshCalls;

// what one world draw needs from the running render call
pub struct WorldFrame<'a> {
    pub calls: &'a MeshCalls,
    // the `ScreenContext&` of the `renderLevel` call
    pub screen: *mut c_void,
    // the `ClientInstance*` of the last tick, for the white texture; null before the first tick
    pub client: *mut c_void,
    // the depth-tested name-tag material, a `mce::MaterialPtr const*`
    pub material: *const c_void,
    // the camera position of the frame's first view
    pub camera: [f32; 3],
}
