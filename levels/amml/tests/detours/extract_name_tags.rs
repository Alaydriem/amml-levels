use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use alaydriem_levels::detours::extract_name_tags;

static SEEN_THIS: AtomicUsize = AtomicUsize::new(0);
static SEEN_RET: AtomicUsize = AtomicUsize::new(0);
static SEEN_SCREEN: AtomicUsize = AtomicUsize::new(0);

extern "C" fn game_extract(this: usize, ret: usize, screen_ctx: usize) -> usize {
    SEEN_THIS.store(this, Ordering::SeqCst);
    SEEN_RET.store(ret, Ordering::SeqCst);
    SEEN_SCREEN.store(screen_ctx, Ordering::SeqCst);
    0xABCD_0000_5678
}

#[test]
fn the_detour_passes_this_the_buffer_and_the_screen_on_and_returns_the_original_rax() {
    extract_name_tags::GUARD.original().store(game_extract as *mut c_void, Ordering::SeqCst);

    let result = extract_name_tags::detour(0x1000, 0x2000, 0x3000);

    assert_eq!(result, 0xABCD_0000_5678);
    assert_eq!(SEEN_THIS.load(Ordering::SeqCst), 0x1000);
    assert_eq!(SEEN_RET.load(Ordering::SeqCst), 0x2000);
    assert_eq!(SEEN_SCREEN.load(Ordering::SeqCst), 0x3000);
}
