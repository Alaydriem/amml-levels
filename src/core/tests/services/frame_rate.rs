use levels_core::services::FrameRate;

#[test]
fn fps_closes_after_one_second() {
    let mut rate = FrameRate::new();
    rate.sample(0, 0.0);
    rate.sample(30, 0.5);
    assert!(rate.fps().is_none());
    assert!(rate.frame_ms().is_none());
    rate.sample(60, 1.0);
    assert!((rate.fps().expect("fps") - 60.0).abs() < 0.01);
    assert!((rate.frame_ms().expect("frame_ms") - 16.67).abs() < 0.01);
}

#[test]
fn next_window_opens_at_closing_sample() {
    let mut rate = FrameRate::new();
    rate.sample(0, 0.0);
    rate.sample(60, 1.0);
    rate.sample(90, 1.5);
    assert!((rate.fps().expect("fps") - 60.0).abs() < 0.01);
    rate.sample(150, 2.0);
    assert!((rate.fps().expect("fps") - 90.0).abs() < 0.01);
}

#[test]
fn cost_mean_of_window() {
    let mut rate = FrameRate::new();
    rate.sample(0, 0.0);
    rate.cost(100.0);
    rate.cost(300.0);
    assert!(rate.cost_us().is_none());
    rate.sample(60, 1.0);
    assert!((rate.cost_us().expect("cost") - 200.0).abs() < 0.001);
}

#[test]
fn cost_resets_for_each_window() {
    let mut rate = FrameRate::new();
    rate.sample(0, 0.0);
    rate.cost(100.0);
    rate.sample(60, 1.0);
    rate.cost(500.0);
    rate.sample(120, 2.0);
    assert!((rate.cost_us().expect("cost") - 500.0).abs() < 0.001);
}
