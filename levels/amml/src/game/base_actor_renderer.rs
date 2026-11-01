use std::ffi::c_void;
use std::ptr::NonNull;

use amml_game::{EntryId, GameEntries};
use amml_msvc::{MsvcString, MsvcVec, Optional, SharedPtr};

use crate::game::GameMirror;
use crate::types::LevelsEntry;

/// Layout mirror of the game's `BaseActorRenderer` (26.51 header). The mod makes one only to own the depth-tested
/// name-tag materials: on the game heap, through the game's constructor, once on the game thread, never freed
#[repr(C)]
pub struct BaseActorRenderer {
    _shader_manager: [u8; 0x38],             // 0x00 ActorShaderManager: vtable, three MaterialPtr
    _name_tag_background: SharedPtr<c_void>, // 0x38 mce::MaterialPtr: shared_ptr<RenderMaterialInfo>
    _name_tag_background_with_backface: SharedPtr<c_void>, // 0x48
    _name_tag_text: SharedPtr<c_void>,       // 0x58
    _name_tag_text_with_backface: SharedPtr<c_void>, // 0x68
    depth_tested_name_tag: SharedPtr<c_void>, // 0x78 mDepthTestedNameTagMat
    _depth_tested_name_tag_with_backface: SharedPtr<c_void>, // 0x88
    depth_tested_name_text: SharedPtr<c_void>, // 0x98 mDepthTestedNameTextMat
    _depth_tested_name_text_with_backface: SharedPtr<c_void>, // 0xA8
}

// the arguments of the static `_extractRenderTextObject` that change per label
pub struct TextObjectArgs<'a> {
    pub tessellator: *mut c_void,
    pub text: &'a MsvcString,
    pub widths: &'a MsvcVec<i32>,
    pub position: [f32; 3],
    pub text_color: [f32; 4],
    pub tag_color: [f32; 4],
    pub scale: f32,
    pub rotation: &'a Optional<[f32; 4]>,
}

// static NameTagRenderObject BaseActorRenderer::_extractRenderTextObject(Tessellator&, std::string const&,
// std::vector<int> const&, Vec3 const&, mce::Color const&, MaterialPtr const*, MaterialPtr const*, mce::Color const&,
// float, std::optional<Quaternion> const&, float). A static function that returns a class takes the result buffer
// first, so every other argument moves one place: rcx the buffer, rdx the tessellator, r8 the string, r9 the
// widths, then the stack
type ExtractFn = unsafe extern "C" fn(
    *mut u8,
    *mut c_void,
    *const MsvcString,
    *const MsvcVec<i32>,
    *const [f32; 3],
    *const [f32; 4],
    *const c_void,
    *const c_void,
    *const [f32; 4],
    f32,
    *const Optional<[f32; 4]>,
    f32,
) -> *mut u8;

impl BaseActorRenderer {
    /// makes the renderer: zeroed bytes of the mirror size from the game heap, then the game's constructor. `None`
    /// when the entry is off, `BaseActorRenderer.size` is not the mirror size or the allocation fails. The object is
    /// never freed
    ///
    /// # Safety
    /// `allocate` must be the game heap's allocator, and the call must be on the game thread
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

    /// the object at `address`, made earlier by `create`
    ///
    /// # Safety
    /// `address` must be the address `as_address` gave for a renderer of this game build
    #[allow(unsafe_code)]
    pub unsafe fn from_address(address: usize) -> Option<&'static Self> {
        // SAFETY: the caller guarantees an object made by `create`, which is never freed
        NonNull::new(address as *mut Self).map(|ptr| unsafe { ptr.as_ref() })
    }

    // the address of the object, to keep it between frames
    pub fn as_address(&self) -> usize {
        std::ptr::from_ref(self) as usize
    }

    // the depth-tested tag material, as the pointer `_extractRenderTextObject` takes
    pub fn tag_material(&self) -> *const c_void {
        std::ptr::from_ref(&self.depth_tested_name_tag).cast()
    }

    // the depth-tested text material
    pub fn text_material(&self) -> *const c_void {
        std::ptr::from_ref(&self.depth_tested_name_text).cast()
    }

    /// builds one name-tag object into `result`, which must hold `NameTagRenderObject.size` bytes, with the
    /// depth-tested materials of this renderer. `None` when the entry is off
    ///
    /// # Safety
    /// `result` must be writable for that size; the tessellator, text and widths must be live
    #[allow(unsafe_code)]
    pub unsafe fn extract_render_text_object(
        &self,
        entries: &GameEntries,
        result: *mut u8,
        args: &TextObjectArgs<'_>,
    ) -> Option<()> {
        let function = usize::try_from(entries.get(LevelsEntry::EXTRACT_RENDER_TEXT_OBJECT)?).ok()?;
        if function == 0 {
            return None;
        }
        // SAFETY: the entry is the address of the static function with the signature above
        let call: ExtractFn = unsafe { std::mem::transmute(function as *const c_void) };
        let line_gap = 0.0_f32;
        // SAFETY: the caller guarantees the buffer and the live arguments, which outlive the call; the game copies
        // the string and the widths it keeps
        unsafe {
            call(
                result,
                args.tessellator,
                args.text,
                args.widths,
                &raw const args.position,
                &raw const args.text_color,
                self.tag_material(),
                self.text_material(),
                &raw const args.tag_color,
                args.scale,
                args.rotation,
                line_gap,
            );
        }
        Some(())
    }
}

impl GameMirror for BaseActorRenderer {
    const SIZE_ENTRY: EntryId = LevelsEntry::BASE_ACTOR_RENDERER_SIZE;
}
