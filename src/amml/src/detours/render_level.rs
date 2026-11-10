use amml_sdk::DetourGuard;

use crate::services::LevelsRuntime;

// void LevelRenderer::renderLevel(ScreenContext&, FrameRenderObject const&): the world render. The original runs
// first; the world meshes are then drawn after the game's own level pass, depth tested against it
pub static GUARD: DetourGuard = DetourGuard::new("render_level");

// the signature of the hooked function: this, the ScreenContext and the FrameRenderObject
pub type RenderLevelFn = unsafe extern "C" fn(usize, usize, usize);

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, screen: usize, frame: usize) {
    let original: RenderLevelFn = GUARD.original_fn();
    // SAFETY: `original` is the next function in the chain, with the hooked signature; the arguments are the game's
    unsafe { original(this, screen, frame) };
    if let Some(runtime) = LevelsRuntime::get() {
        runtime.render_world(this, screen, frame);
    }
}
