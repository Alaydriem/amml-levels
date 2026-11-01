use std::f32::consts::FRAC_1_SQRT_2;

const FULL_TURN_DEGREES: f32 = 360.0;

pub struct LabelRotation;

impl LabelRotation {
    // angleAxis(-yaw, Y) * angleAxis(90°, X) as (x, y, z, w): text flat on the top face, upright from the
    // player's facing. Yaw is wrapped first so yaw and yaw + 360 give the same components, not the negated pair.
    pub fn for_yaw(yaw_degrees: f32) -> [f32; 4] {
        let half = -yaw_degrees.rem_euclid(FULL_TURN_DEGREES).to_radians() / 2.0;
        let (s, c) = half.sin_cos();
        [c * FRAC_1_SQRT_2, s * FRAC_1_SQRT_2, -s * FRAC_1_SQRT_2, c * FRAC_1_SQRT_2]
    }
}
