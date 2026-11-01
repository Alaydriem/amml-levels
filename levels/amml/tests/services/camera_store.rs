use alaydriem_levels::services::CameraStore;

#[test]
fn a_camera_needs_both_the_position_and_the_direction() {
    let store = CameraStore::new();
    assert!(store.read().is_none());

    store.observe_position([1.0, 2.0, 3.0]);
    assert!(store.read().is_none());

    store.observe_forward([0.0, 0.0, 2.0]);
    let camera = store.read().unwrap();
    assert!(camera.is_behind([1.0, 2.0, -10.0]));
    assert!(!camera.is_behind([1.0, 2.0, 10.0]));
}

#[test]
fn a_zero_direction_gives_no_camera() {
    let store = CameraStore::new();
    store.observe_position([0.0; 3]);
    store.observe_forward([0.0; 3]);

    assert!(store.read().is_none());
}

#[test]
fn reset_forgets_both_parts() {
    let store = CameraStore::new();
    store.observe_position([0.0; 3]);
    store.observe_forward([0.0, 0.0, 1.0]);

    store.reset();

    assert!(store.read().is_none());
}
