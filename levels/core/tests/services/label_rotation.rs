use levels_core::services::LabelRotation;

const EPSILON: f32 = 1e-5;

fn close(a: &[f32], b: &[f32]) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < EPSILON)
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

// v' = v + 2w(u x v) + 2u x (u x v), with q = (u, w) in (x, y, z, w) order
fn rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let u = [q[0], q[1], q[2]];
    let t = cross(u, v).map(|c| 2.0 * c);
    let ut = cross(u, t);
    [v[0] + q[3] * t[0] + ut[0], v[1] + q[3] * t[1] + ut[1], v[2] + q[3] * t[2] + ut[2]]
}

#[test]
fn yaw_zero_lays_the_text_flat() {
    let half = std::f32::consts::FRAC_PI_4;
    let q = LabelRotation::for_yaw(0.0);

    assert!(close(&q, &[half.sin(), 0.0, 0.0, half.cos()]));
    assert!(close(&rotate(q, [0.0, 0.0, 1.0]), &[0.0, -1.0, 0.0]));
}

#[test]
fn full_turns_give_the_same_rotation() {
    assert!(close(&LabelRotation::for_yaw(90.0), &LabelRotation::for_yaw(450.0)));
    assert!(close(&LabelRotation::for_yaw(-90.0), &LabelRotation::for_yaw(270.0)));
}

#[test]
fn text_turns_with_the_player_and_stays_flat() {
    let q = LabelRotation::for_yaw(90.0);

    assert!(close(&rotate(q, [1.0, 0.0, 0.0]), &[0.0, 0.0, 1.0]));
    assert!(close(&rotate(q, [0.0, 0.0, 1.0]), &[0.0, -1.0, 0.0]));
}
