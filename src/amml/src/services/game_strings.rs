use crate::types::GameString;

/// Game-heap copies of the texts the mod passes into game objects, so no game object holds a pointer into the mod's
/// image, which is gone after an unload. Each text is copied once, on its first use, into a block that is never
/// freed: the game may keep the pointer past a disable. Bound: one block per `GameString`, fixed at compile time
pub struct GameStrings {
    allocate: unsafe extern "C" fn(usize) -> *mut u8,
    copies: [Option<&'static str>; GameString::ALL.len()],
}

impl GameStrings {
    pub fn new(allocate: unsafe extern "C" fn(usize) -> *mut u8) -> Self {
        Self { allocate, copies: [None; GameString::ALL.len()] }
    }

    /// The game-heap copy of `text`; `None` when the heap gave no block
    #[allow(unsafe_code)]
    pub fn get(&mut self, text: GameString) -> Option<&'static str> {
        let slot = &mut self.copies[text as usize];
        if let Some(copy) = *slot {
            return Some(copy);
        }
        let source = text.text();
        // SAFETY: `allocate` is the game heap's allocator, which takes any non-zero size and returns null on failure;
        // every text is non-empty
        let block = unsafe { (self.allocate)(source.len()) };
        if block.is_null() {
            return None;
        }
        // SAFETY: the block holds `source.len()` bytes and is apart from `source`
        unsafe { std::ptr::copy_nonoverlapping(source.as_ptr(), block, source.len()) };
        // SAFETY: the bytes are a copy of a `str`, and the block is never freed, so it lives as long as the process
        let copy = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(block, source.len())) };
        *slot = Some(copy);
        Some(copy)
    }
}
