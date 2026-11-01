use std::ffi::c_void;
use std::ptr::NonNull;

use amml_game::GameEntries;

use crate::game::{GameMirror, NameTagRenderObject, NameTagRenderObjectCollection};

// a `NameTagRenderObjectCollection*` the game made. Only the vector pointers are read and written
pub struct NameTagCollection {
    collection: NonNull<NameTagRenderObjectCollection>,
    // the mirror size, or `None` when `NameTagRenderObject.size` is not that size
    element_size: Option<usize>,
}

impl NameTagCollection {
    /// wraps `base`; `None` for a null pointer
    ///
    /// # Safety
    /// `base` must be null or a live collection of the game build the entries were resolved for, and must stay live
    /// while the wrapper is used
    #[allow(unsafe_code)]
    pub unsafe fn from_raw(base: *mut c_void, entries: &GameEntries) -> Option<Self> {
        let element_size = NameTagRenderObject::fits(entries).then_some(size_of::<NameTagRenderObject>());
        Some(Self { collection: NonNull::new(base.cast::<NameTagRenderObjectCollection>())?, element_size })
    }

    // the size of one `NameTagRenderObject`; `None` when the build does not have the mirrored size
    pub fn element_size(&self) -> Option<usize> {
        self.element_size
    }

    // first, last and end
    #[allow(unsafe_code)]
    pub fn pointers(&self) -> [usize; 3] {
        // SAFETY: `from_raw` guarantees a live collection
        let parts = unsafe { (*self.collection.as_ptr()).name_tags.raw_parts() };
        parts.map(|pointer| pointer as usize)
    }

    // repoints first, last and end
    #[allow(unsafe_code)]
    pub fn set_pointers(&self, pointers: [usize; 3]) {
        let [first, last, end] = pointers.map(|pointer| pointer as *mut NameTagRenderObject);
        // SAFETY: `from_raw` guarantees a live collection; `NameTagList::move_to` passes ordered pointers into a
        // block that is never freed, and the game reads them when it uses the list later in the frame
        unsafe { (*self.collection.as_ptr()).name_tags.set_raw_parts(first, last, end) };
    }
}
