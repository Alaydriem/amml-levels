use amml_sdk::DetourGuard;

use crate::services::LevelsRuntime;

// NameTagRenderObjectCollection LevelRendererPlayer::extractNameTags(ScreenContext&) const, returned by value through
// `ret`. The original runs first and fills the collection; the overlay then appends its labels to it. The original's
// RAX is returned unchanged
pub static GUARD: DetourGuard = DetourGuard::new("extract_name_tags");

// the signature of the hooked function: this, the hidden return buffer and the ScreenContext, the buffer back in RAX
pub type ExtractNameTagsFn = unsafe extern "C" fn(usize, usize, usize) -> usize;

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, ret: usize, screen_ctx: usize) -> usize {
    let original: ExtractNameTagsFn = GUARD.original_fn();
    // SAFETY: `original` is the next function in the chain, with the hooked signature; the arguments are the game's
    let result = unsafe { original(this, ret, screen_ctx) };
    if let Some(runtime) = LevelsRuntime::get() {
        runtime.draw(ret, screen_ctx);
    }
    result
}
