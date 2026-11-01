// how far behind the camera plane a label may sit and still be drawn, so a label straddling the plane or
// at the edge of a wide field of view is not cut early
const BEHIND_MARGIN: f32 = 1.0;
const MIN_DIRECTION_LENGTH: f32 = 1e-4;

// the camera the frame is drawn from: its world position and the unit direction it faces
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraView {
    position: [f32; 3],
    forward: [f32; 3],
}

impl CameraView {
    // forward is the direction the camera faces, of any length; None when it is zero or not finite
    pub fn from_forward(position: [f32; 3], forward: [f32; 3]) -> Option<Self> {
        let length = Self::dot(forward, forward).sqrt();
        let usable = length.is_finite() && length > MIN_DIRECTION_LENGTH && position.iter().all(|c| c.is_finite());
        usable.then(|| Self { position, forward: forward.map(|c| c / length) })
    }

    pub fn is_behind(&self, point: [f32; 3]) -> bool {
        let offset = [point[0] - self.position[0], point[1] - self.position[1], point[2] - self.position[2]];
        Self::dot(offset, self.forward) < -BEHIND_MARGIN
    }

    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }
}
