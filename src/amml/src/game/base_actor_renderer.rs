use std::ffi::c_void;
use std::ptr::NonNull;

use amml_game::{EntryId, GameEntries};
use amml_msvc::SharedPtr;

use crate::game::GameMirror;
use crate::types::LevelsEntry;

/// Layout mirror of the game's `BaseActorRenderer` (26.51 header). The mod makes one only to own the depth-tested
/// name-tag material, which draws the world meshes when Vibrant Visuals is off: on the game heap, through the
/// game's constructor, once, never freed
#[repr(C)]
pub struct BaseActorRenderer {
    _shader_manager: [u8; 0x38],             // 0x00 ActorShaderManager: vtable, three MaterialPtr
    _name_tag_background: SharedPtr<c_void>, // 0x38 mce::MaterialPtr: shared_ptr<RenderMaterialInfo>
    _name_tag_background_with_backface: SharedPtr<c_void>, // 0x48
    _name_tag_text: SharedPtr<c_void>,       // 0x58
    _name_tag_text_with_backface: SharedPtr<c_void>, // 0x68
    depth_tested_name_tag: SharedPtr<c_void>, // 0x78 mDepthTestedNameTagMat
    _depth_tested_name_tag_with_backface: SharedPtr<c_void>, // 0x88
    _depth_tested_name_text: SharedPtr<c_void>, // 0x98 mDepthTestedNameTextMat
    _depth_tested_name_text_with_backface: SharedPtr<c_void>, // 0xA8
}

impl BaseActorRenderer {
    /// makes the renderer: zeroed bytes of the mirror size from the game heap, then the game's constructor. `None`
    /// when the entry is off, `BaseActorRenderer.size` is not the mirror size or the allocation fails. The object is
    /// never freed
    ///
    /// # Safety
    /// `allocate` must be the game heap's allocator, and the call must be on the render thread inside a render call
    #[allow(unsafe_code)]
    pub unsafe fn create(
        entries: &GameEntries,
        allocate: unsafe extern "C" fn(usize) -> *mut u8,
    ) -> Option<&'static Self> {
        let ctor = usize::try_from(entries.get(LevelsEntry::BASE_ACTOR_RENDERER_CTOR)?).ok()?;
        if ctor == 0 || !Self::fits(entries) {
            return None;
        }
        let size = size_of::<Self>();
        // SAFETY: the caller passes the game heap's allocator
        let ptr = NonNull::new(unsafe { allocate(size) })?;
        // SAFETY: the allocation holds `size` bytes
        unsafe { std::ptr::write_bytes(ptr.as_ptr(), 0, size) };
        // SAFETY: the entry is the address of `BaseActorRenderer::BaseActorRenderer()`, a constructor taking this
        // in rcx and returning it in rax
        let call: unsafe extern "C" fn(*mut u8) -> *mut u8 = unsafe { std::mem::transmute(ctor as *const c_void) };
        // SAFETY: `ptr` is a zeroed block of the class size
        unsafe { call(ptr.as_ptr()) };
        // SAFETY: the constructor built the object, the heap block is aligned for it, and it is never freed
        Some(unsafe { ptr.cast::<Self>().as_ref() })
    }

    // the depth-tested name-tag material, as the `MaterialPtr const*` that `renderMesh` takes
    pub fn tag_material(&self) -> *const c_void {
        std::ptr::from_ref(&self.depth_tested_name_tag).cast()
    }
}

impl GameMirror for BaseActorRenderer {
    const SIZE_ENTRY: EntryId = LevelsEntry::BASE_ACTOR_RENDERER_SIZE;
}
