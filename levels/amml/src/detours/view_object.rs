use amml_game::hooks::CreateViewRenderObjectFn;
use amml_sdk::DetourGuard;

use crate::services::LevelsRuntime;

// ViewRenderObject LevelRendererPlayer::createViewRenderObject(ScreenContext&, SubClientId), returned by value through
// `ret`; the sub client id is a byte in r9, passed on whole. The original's RAX is returned unchanged
pub static GUARD: DetourGuard = DetourGuard::new("view_object");

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, ret: usize, screen_ctx: usize, sub_client_id: usize) -> usize {
    let original: CreateViewRenderObjectFn = GUARD.original_fn();
    // SAFETY: `original` is the next function in the chain, with the hooked signature; the arguments are the game's
    let result = unsafe { original(this, ret, screen_ctx, sub_client_id) };
    if let Some(runtime) = LevelsRuntime::get() {
        runtime.observe_view(ret);
    }
    result
}
