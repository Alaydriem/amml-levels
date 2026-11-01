use std::ffi::c_void;
use std::ptr::NonNull;

use amml_game::GameEntries;
use amml_msvc::MsvcHeap;

// what one extraction needs from the running game
pub struct DrawSetup<'a> {
    pub entries: &'a GameEntries,
    // the game heap, for the strings
    pub heap: &'a MsvcHeap,
    // the game heap's allocator, for the renderer object and the ring
    pub allocate: unsafe extern "C" fn(usize) -> *mut u8,
    // the `ClientInstance*` of the last tick, for the font
    pub client: Option<NonNull<c_void>>,
    pub screen: *mut c_void,
}
