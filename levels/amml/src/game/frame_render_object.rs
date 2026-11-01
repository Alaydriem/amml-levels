use std::ffi::c_void;

use amml_game::{GameEntries, RawObject, RenderCamera};

use crate::types::MeshCalls;

/// The game's `FrameRenderObject`, the frame argument of `LevelRenderer::renderLevel`. Render thread, inside the
/// call it belongs to
pub struct FrameRenderObject;

impl FrameRenderObject {
    /// The camera position of the frame's first view; `None` for a null frame or a frame with no view
    ///
    /// # Safety
    /// `frame` must be null or the live `FrameRenderObject` of the running render call, of the build `calls` and
    /// `entries` were resolved for
    #[allow(unsafe_code)]
    pub unsafe fn camera(frame: *const c_void, calls: &MeshCalls, entries: &GameEntries) -> Option<[f32; 3]> {
        if frame.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees a live FrameRenderObject; `mViews` is a std::vector<ViewRenderObject>,
        // its first and last pointers at the entry's offset
        let [first, last] = unsafe { RawObject::read::<[usize; 2]>(frame, calls.frame_views) }?;
        if first == 0 || first == last {
            return None;
        }
        // SAFETY: `first` is the frame's first ViewRenderObject, live for the call
        unsafe { RenderCamera::new(entries).view_position(first as *const c_void) }
    }
}
