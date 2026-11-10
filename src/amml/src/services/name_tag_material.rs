use std::ffi::c_void;

use amml_game::GameEntries;

use crate::game::BaseActorRenderer;

/// The depth-tested name-tag material of one never-freed `BaseActorRenderer`, made on the first frame that asks.
/// A failed make is not tried again; it is logged once
#[derive(Default)]
pub struct NameTagMaterial {
    renderer: Option<&'static BaseActorRenderer>,
    failed: bool,
}

impl NameTagMaterial {
    pub fn new() -> Self {
        Self::default()
    }

    /// the material, making the renderer when it is not made yet. `None` when the make failed
    ///
    /// # Safety
    /// `allocate` must be the game heap's allocator, and the call must be on the render thread inside a render call
    #[allow(unsafe_code)]
    pub unsafe fn material(
        &mut self,
        entries: &GameEntries,
        allocate: unsafe extern "C" fn(usize) -> *mut u8,
    ) -> Option<*const c_void> {
        if self.renderer.is_none() && !self.failed {
            // SAFETY: forwarded to the caller
            self.renderer = unsafe { BaseActorRenderer::create(entries, allocate) };
            if self.renderer.is_none() {
                self.failed = true;
                curia::error!("the name-tag renderer could not be made; nothing is drawn without Vibrant Visuals");
            }
        }
        self.renderer.map(BaseActorRenderer::tag_material)
    }
}
