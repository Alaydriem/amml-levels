use levels_core::services::Telemetry;

const FRAME: f64 = 1.0 / 60.0;
const BUDGET: u64 = 1 << 20;

#[test]
fn disabled_never_reports() {
    let mut telemetry = Telemetry::new(false, BUDGET);

    for i in 0..=300 {
        let time = i as f64 * FRAME;
        telemetry.frame(time, 10);
        assert!(telemetry.take_report(time).is_none());
    }
}

#[test]
fn reports_once_per_two_seconds() {
    let mut telemetry = Telemetry::new(true, BUDGET);
    let mut reports = Vec::new();

    for i in 0..=150 {
        let time = i as f64 * FRAME;
        telemetry.frame(time, 8);
        telemetry.labels(1200);
        reports.extend(telemetry.take_report(time));
    }

    assert_eq!(reports.len(), 1);
    assert!((reports[0].fps - 60.0).abs() < 0.5);
    assert!((reports[0].columns_per_frame - 8.0).abs() < 1e-3);
    assert_eq!(reports[0].labels, 1200);
}

#[test]
fn averages_timing() {
    let mut telemetry = Telemetry::new(true, BUDGET);
    telemetry.frame(0.0, 0);
    telemetry.timing(10.0, 1.0);
    telemetry.timing(30.0, 3.0);
    telemetry.frame(2.0, 0);

    let report = telemetry.take_report(2.0).expect("a report after two seconds");

    assert!((report.sample_us_avg - 20.0).abs() < 1e-3);
    assert!((report.draw_us_avg - 2.0).abs() < 1e-3);
}

#[test]
fn reports_memory_against_the_budget() {
    let mut telemetry = Telemetry::new(true, BUDGET);
    telemetry.memory(4096);
    telemetry.frame(0.0, 0);
    telemetry.frame(2.0, 0);

    let report = telemetry.take_report(2.0).expect("a report after two seconds");

    assert_eq!((report.memory_bytes, report.budget_bytes), (4096, BUDGET));
    assert!(!report.is_over_budget());
    telemetry.memory(BUDGET + 1);
    telemetry.frame(4.0, 0);
    assert!(telemetry.take_report(4.0).expect("second window").is_over_budget());
}

#[test]
fn window_resets_after_report() {
    let mut telemetry = Telemetry::new(true, BUDGET);
    telemetry.frame(0.0, 0);
    telemetry.frame(0.5, 0);
    telemetry.frame(2.0, 0);
    let first = telemetry.take_report(2.0).expect("first window");

    for i in 1..=120 {
        telemetry.frame(2.0 + i as f64 * FRAME, 0);
    }
    let second = telemetry.take_report(4.0).expect("second window");

    assert!((first.max_frame_ms - 1500.0).abs() < 1e-2);
    assert!((second.max_frame_ms - 1000.0 / 60.0).abs() < 1e-2);
}

#[test]
fn report_carries_marker_counters() {
    let mut telemetry = Telemetry::new(true, BUDGET);
    telemetry.frame(0.0, 0);
    telemetry.world(3, 1200);
    telemetry.world_timing(10.0, 1.0, 2);
    telemetry.world_timing(30.0, 3.0, 1);
    telemetry.frame(2.0, 0);

    let report = telemetry.take_report(2.0).expect("a report after two seconds");

    assert_eq!((report.markers, report.quads_drawn, report.meshes_built), (3, 1200, 3));
    assert!((report.world_build_us_avg - 20.0).abs() < 1e-3);
    assert!((report.world_draw_us_avg - 2.0).abs() < 1e-3);
}
