use std::ffi::c_void;
use std::mem::offset_of;

use amml_msvc::SharedPtr;

/// Layout mirror of the game's `mce::TexturePtr` (32 bytes): two `shared_ptr`, the texture data and the resource
/// location. The pointer pairs are kept raw so that the mirror is `Copy` and fits `MsvcVariant`. A plain copy takes
/// no use: only the owner of the uses the game wrote releases them, once
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TexturePtrLayout {
    client_texture: [*const c_void; 2],    // 0x00 mClientTexture
    resource_location: [*const c_void; 2], // 0x10 mResourceLocationPtr
}

impl TexturePtrLayout {
    // the `std::variant<monostate, TexturePtr, ClientTexture, ServerTexture>` index of a `TexturePtr`
    pub const VARIANT_INDEX: u8 = 1;

    // a texture with both pointers empty, for the game to fill
    pub const EMPTY: TexturePtrLayout =
        TexturePtrLayout { client_texture: [std::ptr::null(); 2], resource_location: [std::ptr::null(); 2] };

    // whether the game left the texture pointer empty
    pub fn is_empty(&self) -> bool {
        self.client_texture[0].is_null()
    }

    /// Gives back the use of each pointer, as the `TexturePtr` destructor does, and leaves both empty
    ///
    /// # Safety
    /// The value must hold the uses it gives back, as one the game filled does; a plain byte copy of it must not be
    /// released as well
    #[allow(unsafe_code)]
    pub unsafe fn release(&mut self) {
        // SAFETY: SharedPtr is #[repr(C)] {element pointer, control block pointer}, the same size and alignment as
        // the two raw pointers; the caller guarantees the uses
        unsafe {
            (*self.resource_location.as_mut_ptr().cast::<SharedPtr<c_void>>()).release();
            (*self.client_texture.as_mut_ptr().cast::<SharedPtr<c_void>>()).release();
        }
    }
}

const _: () = assert!(offset_of!(TexturePtrLayout, resource_location) == 0x10);
const _: () = assert!(size_of::<TexturePtrLayout>() == 32);
const _: () = assert!(size_of::<SharedPtr<c_void>>() == 16);
