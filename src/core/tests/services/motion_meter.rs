use levels_core::services::MotionMeter;
use levels_core::structs::world::Vec3f;

struct Fixture;

impl Fixture {
    fn at(x: f32, y: f32, z: f32) -> Vec3f {
        Vec3f { x, y, z }
    }
}

#[test]
fn steady_walk_speed() {
    let mut meter = MotionMeter::new();
    for i in 0..12 {
        meter.sample(Fixture::at(0.1 * i as f32, 64.0, 0.0), 0, 0.05 * i as f64);
    }
    let (horizontal, total) = meter.speed().expect("speed after steps");
    assert!((horizontal - 2.0).abs() < 0.01, "horizontal {horizontal}");
    assert!((total - 2.0).abs() < 0.01, "total {total}");
}

#[test]
fn no_speed_before_one_step() {
    let mut meter = MotionMeter::new();
    assert!(meter.speed().is_none());
    meter.sample(Fixture::at(0.0, 64.0, 0.0), 0, 0.0);
    assert!(meter.speed().is_none());
}

#[test]
fn vertical_only_speed() {
    let mut meter = MotionMeter::new();
    for i in 0..6 {
        meter.sample(Fixture::at(5.0, 64.0 - i as f32, 5.0), 0, 0.05 * i as f64);
    }
    let (horizontal, total) = meter.speed().expect("speed after steps");
    assert!(horizontal.abs() < 0.001, "horizontal {horizontal}");
    assert!((total - 20.0).abs() < 0.01, "total {total}");
}

#[test]
fn step_over_limit_resets() {
    let mut meter = MotionMeter::new();
    meter.sample(Fixture::at(0.0, 64.0, 0.0), 0, 0.0);
    meter.sample(Fixture::at(0.1, 64.0, 0.0), 0, 0.05);
    assert!(meter.speed().is_some());
    meter.sample(Fixture::at(17.1, 64.0, 0.0), 0, 0.10);
    assert!(meter.speed().is_none());
    meter.sample(Fixture::at(17.2, 64.0, 0.0), 0, 0.15);
    meter.sample(Fixture::at(17.3, 64.0, 0.0), 0, 0.20);
    let (horizontal, _) = meter.speed().expect("speed after reset");
    assert!((horizontal - 2.0).abs() < 0.01, "horizontal {horizontal}");
}

#[test]
fn dimension_change_resets() {
    let mut meter = MotionMeter::new();
    meter.sample(Fixture::at(0.0, 64.0, 0.0), 0, 0.0);
    meter.sample(Fixture::at(0.1, 64.0, 0.0), 0, 0.05);
    assert!(meter.speed().is_some());
    meter.sample(Fixture::at(0.2, 64.0, 0.0), 1, 0.10);
    assert!(meter.speed().is_none());
    meter.sample(Fixture::at(0.3, 64.0, 0.0), 1, 0.15);
    assert!(meter.speed().is_some());
}

#[test]
fn zero_time_step_ignored() {
    let mut meter = MotionMeter::new();
    meter.sample(Fixture::at(0.0, 64.0, 0.0), 0, 1.0);
    meter.sample(Fixture::at(0.1, 64.0, 0.0), 0, 1.0);
    assert!(meter.speed().is_none());
    meter.sample(Fixture::at(0.2, 64.0, 0.0), 0, 1.05);
    let (horizontal, total) = meter.speed().expect("speed after valid step");
    assert!(horizontal.is_finite() && total.is_finite());
    assert!((horizontal - 2.0).abs() < 0.01, "horizontal {horizontal}");
}

#[test]
fn mean_covers_last_ten_steps_only() {
    let mut meter = MotionMeter::new();
    let mut x = 0.0_f32;
    let mut time = 0.0_f64;
    meter.sample(Fixture::at(x, 64.0, 0.0), 0, time);
    // five fast steps, then ten slow steps push the fast ones out of the ring
    for _ in 0..5 {
        x += 1.0;
        time += 0.05;
        meter.sample(Fixture::at(x, 64.0, 0.0), 0, time);
    }
    for _ in 0..10 {
        x += 0.1;
        time += 0.05;
        meter.sample(Fixture::at(x, 64.0, 0.0), 0, time);
    }
    let (horizontal, _) = meter.speed().expect("speed");
    assert!((horizontal - 2.0).abs() < 0.01, "horizontal {horizontal}");
}

#[test]
fn reset_clears_speed() {
    let mut meter = MotionMeter::new();
    meter.sample(Fixture::at(0.0, 64.0, 0.0), 0, 0.0);
    meter.sample(Fixture::at(0.1, 64.0, 0.0), 0, 0.05);
    meter.reset();
    assert!(meter.speed().is_none());
}
