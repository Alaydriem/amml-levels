// the storage of the grown name-tag list. The mod cannot call the game's allocator for the list, so it keeps a ring of
// SLOTS buffers, one per frame in flight: the game destroys the elements of a frame's list but never frees its
// storage, so a slot is reused SLOTS frames later. A slot only grows. The buffers come from the game heap and are
// never freed: the game may still hold a list in a slot after the mod is disabled, and no pointer into the mod's own
// image goes into a game object. The ring takes an allocator and no deallocator, so nothing here can free a slot
pub struct LabelRing {
    allocate: unsafe extern "C" fn(usize) -> *mut u8,
    slots: [Slot; Self::SLOTS],
    next: usize,
}

#[derive(Clone, Copy)]
struct Slot {
    ptr: *mut u8,
    capacity: usize,
}

impl LabelRing {
    pub const SLOTS: usize = 8;

    // blocks from the game heap are 16-byte aligned; capacities are rounded to the same
    const ALIGN: usize = 16;

    pub fn new(allocate: unsafe extern "C" fn(usize) -> *mut u8) -> Self {
        Self { allocate, slots: [Slot { ptr: std::ptr::null_mut(), capacity: 0 }; Self::SLOTS], next: 0 }
    }

    // the next slot, holding room for at least `bytes`. A slot that is too small gets a new block of twice its
    // size, or `bytes` when that is more; the old block stays allocated, since the game may read it. `None` when
    // the heap returns null; the slot is then left as it was
    #[allow(unsafe_code)]
    pub fn next(&mut self, bytes: usize) -> Option<*mut u8> {
        let index = self.next;
        self.next = (index + 1) % Self::SLOTS;
        let slot = self.slots[index];
        if !slot.ptr.is_null() && slot.capacity >= bytes {
            return Some(slot.ptr);
        }
        let capacity = bytes.max(slot.capacity.saturating_mul(2)).max(Self::ALIGN).next_multiple_of(Self::ALIGN);
        // SAFETY: `allocate` is the game heap's allocator, which takes any non-zero size and returns null on
        // failure
        let ptr = unsafe { (self.allocate)(capacity) };
        if ptr.is_null() {
            return None;
        }
        self.slots[index] = Slot { ptr, capacity };
        Some(ptr)
    }

    // the bytes the slots hold now, for telemetry
    pub fn resident_bytes(&self) -> u64 {
        self.slots.iter().map(|slot| slot.capacity as u64).sum()
    }
}
