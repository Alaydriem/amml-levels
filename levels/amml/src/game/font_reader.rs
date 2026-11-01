use std::ffi::c_void;
use std::mem::MaybeUninit;

use amml_game::{GameEntries, RawObject};

use crate::game::{FontHandle, GameMirror, StringView};
use crate::types::LevelsEntry;

// the font reads: the width of a text in the game font
pub struct FontReader;

impl FontReader {
    // the font size the widths are measured at
    const FONT_SIZE: f32 = 1.0;

    /// the widths of `texts` in the game font, one per text. The client is asked for its `FontHandle` (a class
    /// returned by value through a hidden pointer), the font is picked from it, each text is measured through the
    /// `getLineLength` virtual, and the handle is destroyed the way the game destroys it. `None` when an entry is
    /// off, `FontHandle.size` is not the size of the mirror, a pointer is null or a call has no code
    ///
    /// # Safety
    /// `client` must be a live `ClientInstance` of the game build the entries were resolved for, and the call must be
    /// on the game thread
    #[allow(unsafe_code)]
    pub unsafe fn measure(entries: &GameEntries, client: *mut c_void, texts: &[&str]) -> Option<Vec<i32>> {
        let get_handle_slot = entries.get(LevelsEntry::GET_FONT_HANDLE)?;
        let destroy = usize::try_from(entries.get(LevelsEntry::FONT_HANDLE_DTOR)?).ok()?;
        let line_length_slot = entries.get(LevelsEntry::FONT_GET_LINE_LENGTH)?;
        // every check is made before the handle exists, so that no early return leaves a handle undestroyed
        if destroy == 0 || client.is_null() || !FontHandle::fits(entries) {
            return None;
        }
        // SAFETY: the caller guarantees a live client, whose first qword is its vtable, and the entry is the slot of
        // `getFontHandle` in it
        let get_handle = unsafe { RawObject::slot(client, get_handle_slot) }?;
        // SAFETY: the slot holds `FontHandle ClientInstance::getFontHandle() const`: this in rcx, the hidden
        // return buffer in rdx
        let get_handle: unsafe extern "C" fn(*mut c_void, *mut FontHandle) -> *mut c_void =
            unsafe { std::mem::transmute(get_handle) };
        let mut handle = MaybeUninit::<FontHandle>::zeroed();
        let handle_ptr = handle.as_mut_ptr();
        // SAFETY: the buffer is a FontHandle, whose size `fits` checked against the build, and the client is live
        unsafe { get_handle(client, handle_ptr) };
        // SAFETY: the handle was filled by the game just now, and its repository is the game's
        let font = unsafe { (*handle_ptr).font() };
        let widths = font.and_then(|font| {
            // SAFETY: the font is live while the handle is, and the entry is the slot of `getLineLength` in its vtable
            let function = unsafe { RawObject::slot(font, line_length_slot) }?;
            // SAFETY: the slot holds `int Font::getLineLength(std::string_view, float, bool)`: this in rcx, a
            // pointer to the string_view copy in rdx, the size in xmm2, the bool in r9
            let measure: unsafe extern "C" fn(*mut c_void, *const StringView, f32, u8) -> i32 =
                unsafe { std::mem::transmute(function) };
            Some(
                texts
                    .iter()
                    .map(|text| {
                        let view = StringView::of(text);
                        // SAFETY: the view lives for the call, and the font is live
                        unsafe { measure(font, &raw const view, Self::FONT_SIZE, 0) }
                    })
                    .collect::<Vec<i32>>(),
            )
        });
        // SAFETY: the entry is the address of the non-deleting `FontHandle::~FontHandle()`, which takes this in rcx
        let destroy: unsafe extern "C" fn(*mut FontHandle) = unsafe { std::mem::transmute(destroy as *const c_void) };
        // SAFETY: the handle was filled by the game and is destroyed exactly once
        unsafe { destroy(handle_ptr) };
        widths
    }
}
