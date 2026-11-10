use std::alloc::{Layout, alloc, dealloc};
use std::sync::atomic::{AtomicUsize, Ordering};

use amml_msvc::MsvcHeap;

static FREED: AtomicUsize = AtomicUsize::new(0);

// a heap made of the Rust allocator, with the same two function shapes as the game operator new and delete
pub struct FakeHeap;

impl FakeHeap {
    pub unsafe extern "C" fn allocate(size: usize) -> *mut u8 {
        // SAFETY: the callers ask for a non-zero size
        unsafe { alloc(Layout::from_size_align(size, 16).unwrap()) }
    }

    pub unsafe extern "C" fn free(ptr: *mut u8, size: usize) {
        FREED.fetch_add(1, Ordering::SeqCst);
        // SAFETY: `ptr` came from `allocate` with this size
        unsafe { dealloc(ptr, Layout::from_size_align(size, 16).unwrap()) };
    }

    pub fn msvc() -> MsvcHeap {
        MsvcHeap::new(Self::allocate, Self::free)
    }

    // the number of blocks `free` has been given in this process
    pub fn frees() -> usize {
        FREED.load(Ordering::SeqCst)
    }
}
