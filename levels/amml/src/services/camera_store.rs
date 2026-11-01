use std::sync::{Mutex, PoisonError};

use levels_core::structs::draw::CameraView;

// the render camera of the last frame: the position from the view object and the direction from the camera. Two
// render hooks write them and the name-tag hook reads them, so a lock holds the pair
pub struct CameraStore {
    state: Mutex<CameraState>,
}

#[derive(Clone, Copy)]
struct CameraState {
    position: Option<[f32; 3]>,
    forward: Option<[f32; 3]>,
}

impl CameraStore {
    pub const fn new() -> Self {
        Self { state: Mutex::new(CameraState { position: None, forward: None }) }
    }

    pub fn observe_position(&self, position: [f32; 3]) {
        self.lock().position = Some(position);
    }

    pub fn observe_forward(&self, forward: [f32; 3]) {
        self.lock().forward = Some(forward);
    }

    // the camera, when both parts were seen and the direction is usable
    pub fn read(&self) -> Option<CameraView> {
        let state = *self.lock();
        CameraView::from_forward(state.position?, state.forward?)
    }

    // forgets both parts, as when the mod is disabled
    pub fn reset(&self) {
        let mut state = self.lock();
        state.position = None;
        state.forward = None;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, CameraState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for CameraStore {
    fn default() -> Self {
        Self::new()
    }
}
