use crate::errors::LayoutFault;
use crate::game::NameTagCollection;

// the game's name-tag list, checked and ready to be grown. Opening it reads the three vector pointers and refuses a
// layout the mod does not know: the draw then leaves the game's list alone
pub struct NameTagList {
    collection: NameTagCollection,
    first: usize,
    last: usize,
    element_size: usize,
}

impl NameTagList {
    // reads the list. The pointers must satisfy first <= last <= end, with first null only for an empty list, and
    // the bytes between first and last must be a whole number of elements
    pub fn open(collection: NameTagCollection) -> Result<Self, LayoutFault> {
        let Some(element_size) = collection.element_size() else {
            return Err(LayoutFault::ElementSize);
        };
        let [first, last, end] = collection.pointers();
        let ordered = first <= last && last <= end && (first != 0 || last == 0);
        if !ordered {
            return Err(LayoutFault::Unordered);
        }
        if !(last - first).is_multiple_of(element_size) {
            return Err(LayoutFault::Misaligned);
        }
        Ok(Self { collection, first, last, element_size })
    }

    // the number of tags the game made
    pub fn len(&self) -> usize {
        (self.last - self.first) / self.element_size
    }

    pub fn is_empty(&self) -> bool {
        self.first == self.last
    }

    pub fn element_size(&self) -> usize {
        self.element_size
    }

    /// copies the game's tags bit for bit to `storage`, which must hold `len() + added` elements, and repoints the
    /// list there: first at `storage`, last and end after the `added` new slots. The first new slot is returned. The
    /// old storage is left as it is; the game never frees it
    ///
    /// # Safety
    /// `storage` must be writable for `(len() + added) * element_size()` bytes, must not overlap the game's own
    /// storage, and must stay allocated for as long as the game may read the list
    #[allow(unsafe_code)]
    pub unsafe fn move_to(&mut self, storage: *mut u8, added: usize) -> *mut u8 {
        let existing = self.last - self.first;
        if existing > 0 {
            // SAFETY: the game's storage holds `existing` bytes from `first`; the caller guarantees `storage` is
            // writable for them and apart from it
            unsafe { std::ptr::copy_nonoverlapping(self.first as *const u8, storage, existing) };
        }
        let end = (storage as usize) + existing + added * self.element_size;
        self.collection.set_pointers([storage as usize, end, end]);
        self.first = storage as usize;
        self.last = end;
        storage.wrapping_add(existing)
    }
}
