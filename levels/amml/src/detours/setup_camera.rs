use amml_game::hooks::SetupCameraFn;
use amml_sdk::DetourGuard;

use crate::services::LevelsRuntime;

// void LevelRendererPlayer::setupCamera(mce::Camera&, float): once per frame. Read after the
// original and after the mods that wrote the camera before this one
pub static GUARD: DetourGuard = DetourGuard::new("setup_camera");

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, camera: usize, alpha: f32) {
    let original: SetupCameraFn = GUARD.original_fn();
    // SAFETY: `original` is the next function in the chain, with the hooked signature; the arguments are the game's
    unsafe { original(this, camera, alpha) };
    if let Some(runtime) = LevelsRuntime::get() {
        runtime.observe_camera(camera);
    }
}
