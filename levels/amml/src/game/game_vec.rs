use amml_msvc::{MsvcError, MsvcHeap, MsvcVec};

/// Edits a game `std::vector` in place: growth on the game heap with the MSVC STL's rule (the capacity grows by
/// half, or to the size asked for when that is more), so the game's own code can go on using and freeing the vector
pub struct GameVec;

impl GameVec {
    /// Makes room for `total` elements; the elements stay as they were. The old block, if any, is freed once after
    /// the copy
    ///
    /// # Safety
    /// The vector's block must have come from `heap` (or be null), and `heap` must be the game's allocator
    #[allow(unsafe_code)]
    pub unsafe fn reserve<T: Copy>(heap: &MsvcHeap, vec: &mut MsvcVec<T>, total: usize) -> Result<(), MsvcError> {
        let [first, _, end] = vec.raw_parts();
        let len = vec.len();
        let capacity = if first.is_null() {
            0
        } else {
            // SAFETY: a non-null vector has `first <= end` inside one block
            unsafe { end.offset_from(first) as usize }
        };
        if total <= capacity {
            return Ok(());
        }
        let grown = capacity + capacity / 2;
        let new_capacity = if grown < total { total } else { grown };
        let bytes = new_capacity * size_of::<T>();
        // SAFETY: the caller guarantees the game's allocator
        let block = unsafe { heap.allocate(bytes) }.cast::<T>();
        if block.is_null() {
            return Err(MsvcError::AllocationFailed { bytes });
        }
        // SAFETY: the new block has room for `new_capacity >= len` elements and does not overlap the old one; the
        // old block came from `heap` for `capacity` elements and is not used again
        unsafe {
            if !first.is_null() {
                std::ptr::copy_nonoverlapping(first, block, len);
                heap.deallocate(first.cast(), capacity * size_of::<T>());
            }
            vec.set_raw_parts(block, block.add(len), block.add(new_capacity));
        }
        Ok(())
    }

    /// Appends `count` elements, element `index` being `item(index)`, growing the vector when its capacity is short.
    /// No buffer of the mod's own is made
    ///
    /// # Safety
    /// As [`GameVec::reserve`]
    #[allow(unsafe_code)]
    pub unsafe fn extend_with<T: Copy>(
        heap: &MsvcHeap,
        vec: &mut MsvcVec<T>,
        count: usize,
        mut item: impl FnMut(usize) -> T,
    ) -> Result<(), MsvcError> {
        if count == 0 {
            return Ok(());
        }
        let len = vec.len();
        // SAFETY: forwarded to the caller
        unsafe { Self::reserve(heap, vec, len + count) }?;
        let [first, _, end] = vec.raw_parts();
        // SAFETY: the reserve left room for `len + count` elements from `first`; the elements are Copy, so a write
        // drops nothing
        unsafe {
            for index in 0..count {
                first.add(len + index).write(item(index));
            }
            vec.set_raw_parts(first, first.add(len + count), end);
        }
        Ok(())
    }
}
