use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use alaydriem_levels::detours::render_level;

static CALLS: AtomicUsize = AtomicUsize::new(0);
static SEEN_THIS: AtomicUsize = AtomicUsize::new(0);
static SEEN_SCREEN: AtomicUsize = AtomicUsize::new(0);
static SEEN_FRAME: AtomicUsize = AtomicUsize::new(0);

extern "C" fn game_render_level(this: usize, screen: usize, frame: usize) {
    CALLS.fetch_add(1, Ordering::SeqCst);
    SEEN_THIS.store(this, Ordering::SeqCst);
    SEEN_SCREEN.store(screen, Ordering::SeqCst);
    SEEN_FRAME.store(frame, Ordering::SeqCst);
}

#[test]
fn detour_runs_the_original_first() {
    render_level::GUARD.original().store(game_render_level as *mut c_void, Ordering::SeqCst);

    render_level::detour(0x1000, 0x2000, 0x3000);

    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(SEEN_THIS.load(Ordering::SeqCst), 0x1000);
    assert_eq!(SEEN_SCREEN.load(Ordering::SeqCst), 0x2000);
    assert_eq!(SEEN_FRAME.load(Ordering::SeqCst), 0x3000);
}
