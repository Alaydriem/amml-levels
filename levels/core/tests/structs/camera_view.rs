use levels_core::structs::draw::CameraView;

const EAST: [f32; 3] = [1.0, 0.0, 0.0];

fn looking_east() -> CameraView {
    CameraView::from_forward([0.0, 65.6, 0.0], EAST).expect("a camera with a direction")
}

#[test]
fn camera_without_a_direction_gives_none() {
    assert!(CameraView::from_forward([0.0, 65.0, 0.0], [0.0, 0.0, 0.0]).is_none());
    assert!(CameraView::from_forward([0.0, 65.0, 0.0], [f32::NAN, 0.0, 0.0]).is_none());
    assert!(CameraView::from_forward([f32::INFINITY, 65.0, 0.0], EAST).is_none());
}

#[test]
fn point_in_front_is_not_behind() {
    assert!(!looking_east().is_behind([10.0, 64.0, 3.0]));
}

#[test]
fn point_behind_past_the_margin_is_behind() {
    assert!(looking_east().is_behind([-5.0, 64.0, 0.0]));
}

#[test]
fn point_beside_or_just_behind_the_camera_is_kept() {
    let camera = looking_east();

    assert!(!camera.is_behind([0.0, 64.0, 8.0]));
    assert!(!camera.is_behind([-0.5, 64.0, 2.0]));
}

#[test]
fn forward_length_does_not_change_the_answer() {
    let long = CameraView::from_forward([0.0, 65.6, 0.0], [100.0, 0.0, 0.0]).expect("a camera with a direction");

    assert_eq!(long.is_behind([-5.0, 64.0, 0.0]), looking_east().is_behind([-5.0, 64.0, 0.0]));
    assert_eq!(long.is_behind([-0.5, 64.0, 0.0]), looking_east().is_behind([-0.5, 64.0, 0.0]));
}

#[test]
fn looking_straight_down_keeps_everything_below() {
    let camera = CameraView::from_forward([0.0, 70.0, 0.0], [0.0, -1.0, 0.0]).expect("a camera with a direction");

    assert!(!camera.is_behind([-20.0, 64.0, 15.0]));
    assert!(camera.is_behind([0.0, 80.0, 0.0]));
}
