use std::ffi::c_void;

use amml_msvc::{MsvcVec, SharedPtr};

/// Layout mirror of the start of the game's `FontRepository` (26.51 header). It is reached only through the pointer
/// in a `FontHandle`, so the members after the font vector are left out
#[repr(C)]
pub struct FontRepository {
    _bases: [u8; 0x38],    // 0x00 AppPlatformListener (0x20), EnableNonOwnerReferences (0x18)
    _is_initialized: bool, // 0x38 mIsInitialized
    loaded_fonts: MsvcVec<SharedPtr<c_void>>, // 0x40 mLoadedFonts: vector<gsl::not_null<shared_ptr<Font>>>
}

impl FontRepository {
    // the loaded fonts; a font id is an index into them
    pub fn fonts(&self) -> &[SharedPtr<c_void>] {
        self.loaded_fonts.as_slice()
    }
}
