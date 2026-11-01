use std::ffi::c_void;

use amml_game::EntryId;
use amml_msvc::SharedPtr;

use crate::game::{FontRepository, GameMirror};
use crate::types::LevelsEntry;

/// Layout mirror of the game's `FontHandle` (26.51 header), which `ClientInstance::getFontHandle` returns by value
#[repr(C)]
pub struct FontHandle {
    _base: [u8; 0x18],                 // 0x00 EnableNonOwnerReferences: vtable, control block
    _repository_control: [u8; 0x10],   // 0x18 mFontRepository (NonOwnerPointer): control block
    repository: *const FontRepository, // 0x28 mFontRepository (NonOwnerPointer): the pointer
    default_font: SharedPtr<c_void>,   // 0x30 mDefaultFont: shared_ptr<Font>
    font_id: u64,                      // 0x40 mFontId
    _is_dummy_handle: bool,            // 0x48 mIsDummyHandle
}

impl FontHandle {
    /// The `Font` of the handle, picked the way `FontHandle::getFont` picks it (26.60 inlines that function, so the
    /// mod does the same reads): the font at index `mFontId` of the repository when the id is below the font count
    /// (the invalid id is all ones and never is), otherwise the default font. `None` when the picked pointer is null
    ///
    /// # Safety
    /// The handle must have been filled by the game, and its repository must be live
    #[allow(unsafe_code)]
    pub unsafe fn font(&self) -> Option<*mut c_void> {
        // SAFETY: the caller guarantees that the repository pointer is null or a live repository
        let repository = unsafe { self.repository.as_ref() };
        let picked = repository
            .and_then(|repository| usize::try_from(self.font_id).ok().and_then(|index| repository.fonts().get(index)));
        match picked {
            Some(font) => Self::pointer(font),
            None => Self::pointer(&self.default_font),
        }
    }

    fn pointer(font: &SharedPtr<c_void>) -> Option<*mut c_void> {
        font.get().map(|font| std::ptr::from_ref(font).cast_mut())
    }
}

impl GameMirror for FontHandle {
    const SIZE_ENTRY: EntryId = LevelsEntry::FONT_HANDLE_SIZE;
}
