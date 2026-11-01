use levels_core::config::{LevelsConfig, ScanConfig};
use levels_core::services::{LabelRotation, LevelsController};
use levels_core::structs::draw::CameraView;
use levels_core::structs::frame::FrameStatus;
use levels_core::structs::light::SurfaceSample;
use levels_core::structs::manager::ManagerSignal;
use levels_core::structs::world::{BlockPos, ColumnPos};

use crate::support::frames::{feet, in_world, out_of_world};

const BYTES_PER_MB: u64 = 1024 * 1024;

fn controller() -> LevelsController {
    LevelsController::new(LevelsConfig::default())
}

// true when the press flipped the overlay
fn press(controller: &mut LevelsController) -> bool {
    let was_on = controller.is_on();
    controller.signal(ManagerSignal::TogglePressed);
    controller.is_on() != was_on
}

fn turned_on_with(config: LevelsConfig) -> LevelsController {
    let mut controller = LevelsController::new(config);
    controller.frame(in_world(0));
    assert!(press(&mut controller));
    controller
}

fn turned_on() -> LevelsController {
    turned_on_with(LevelsConfig::default())
}

fn sample(x: i32, y: i32, z: i32) -> SurfaceSample {
    SurfaceSample { pos: BlockPos { x, y, z }, block_light: 7, sky_light: 0 }
}

fn submit_under_feet(controller: &mut LevelsController) {
    controller.submit_column(feet().column(), &[sample(feet().x, feet().y - 1, feet().z)]);
}

// telemetry on; frames run from `from` for just over one 2 s window
fn run_window(controller: &mut LevelsController, from: u32) {
    for i in from..=from + 121 {
        controller.frame(in_world(i));
    }
}

#[test]
fn toggle_turns_on_and_off() {
    let mut controller = controller();
    controller.frame(in_world(0));

    assert!(press(&mut controller));
    assert!(controller.is_on());
    assert!(press(&mut controller));
    assert!(!controller.is_on());
}

#[test]
fn toggle_signal_with_menu_open_stays_off() {
    let mut controller = controller();
    controller.frame(FrameStatus { menu_open: true, ..in_world(0) });

    assert!(!press(&mut controller));
    assert!(!controller.is_on());
}

#[test]
fn press_in_a_menu_does_not_block_the_next_press() {
    let mut controller = controller();
    controller.frame(FrameStatus { menu_open: true, ..in_world(0) });
    assert!(!press(&mut controller));

    controller.frame(in_world(1));

    assert!(press(&mut controller));
    assert!(controller.is_on());
}

#[test]
fn toggle_signal_without_world_stays_off() {
    let mut controller = controller();

    assert!(!press(&mut controller));
    assert!(!controller.is_on());
}

#[test]
fn lease_lost_hides_light_but_keeps_toggle() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);

    controller.signal(ManagerSignal::LeaseLost);

    assert!(controller.is_on());
    assert!(controller.frame(in_world(2)).is_empty());
    assert!(controller.draw_list(0, None).is_empty());

    controller.signal(ManagerSignal::LeaseRestored);
    controller.frame(in_world(3));

    assert_eq!(controller.draw_list(0, None).len(), 1);
}

#[test]
fn lease_suspended_hides_world_plan_until_restored() {
    let mut controller = turned_on();
    controller.signal(ManagerSignal::BordersPressed);
    controller.frame(in_world(1));
    assert!(controller.world_plan().is_some());

    controller.signal(ManagerSignal::LeaseSuspended);
    assert!(controller.world_plan().is_none());

    controller.signal(ManagerSignal::LeaseRestored);
    assert!(controller.world_plan().is_some());
}

#[test]
fn wants_world_draw_with_only_markers() {
    let mut controller = controller();
    controller.frame(in_world(0));
    assert!(!controller.wants_world_draw());

    controller.signal(ManagerSignal::SphereDropPressed);

    assert!(!controller.is_on());
    assert!(controller.wants_world_draw());

    controller.signal(ManagerSignal::SphereClearPressed);

    assert!(!controller.wants_world_draw());
}

#[test]
fn disabled_clears_markers_and_lease_hiding() {
    let mut controller = turned_on();
    controller.signal(ManagerSignal::BordersPressed);
    controller.signal(ManagerSignal::SphereDropPressed);
    controller.signal(ManagerSignal::LeaseSuspended);

    controller.signal(ManagerSignal::Disabled);
    controller.frame(in_world(1));

    assert!(!controller.wants_world_draw());
    assert!(controller.world_plan().is_none());
    controller.signal(ManagerSignal::BordersPressed);
    controller.frame(in_world(2));
    assert!(controller.world_plan().is_some(), "the lease hiding is cleared");
}

#[test]
fn disabled_turns_off() {
    let mut controller = turned_on();

    controller.signal(ManagerSignal::Disabled);

    assert!(!controller.is_on());
}

#[test]
fn apply_config_changes_label_colors_live() {
    let mut controller = turned_on();
    submit_under_feet(&mut controller);
    let blue = [0.0, 0.0, 1.0, 1.0];
    let mut config = LevelsConfig::default();
    config.label.yellow = blue;

    controller.apply_config(config);

    let labels = controller.draw_list(0, None);
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].color, blue);
}

#[test]
fn apply_config_sanitizes_light_bands() {
    let mut controller = turned_on();
    submit_under_feet(&mut controller);
    let mut config = LevelsConfig::default();
    config.light.red_max = 9;
    config.light.yellow_max = 3;

    controller.apply_config(config);

    let labels = controller.draw_list(0, None);
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].color, LevelsConfig::default().label.red);
}

#[test]
fn apply_config_keeps_scan_until_restart() {
    let mut controller = turned_on();
    let before = controller.max_columns_per_frame();
    let mut config = LevelsConfig::default();
    config.scan.radius = 2;
    config.scan.max_columns_per_frame = 1;

    controller.apply_config(config);

    assert_eq!(controller.max_columns_per_frame(), before);
    assert!(controller.frame(in_world(30)).len() > 1);
}

#[test]
fn off_requests_nothing() {
    let mut controller = controller();

    assert!(controller.frame(in_world(0)).is_empty());
    submit_under_feet(&mut controller);
    assert!(controller.draw_list(0, None).is_empty());
}

#[test]
fn on_requests_columns() {
    let mut controller = turned_on();

    assert!(!controller.frame(in_world(1)).is_empty());
}

#[test]
fn big_time_step_samples_at_most_the_frame_limit() {
    let mut controller = turned_on();
    controller.frame(in_world(1));

    let requests = controller.frame(FrameStatus { time_seconds: 60.0, ..in_world(2) }).len();

    assert!(requests <= ScanConfig::default().max_columns_per_frame as usize);
}

#[test]
fn submitted_column_draws() {
    let mut controller = turned_on();
    controller.frame(in_world(1));

    submit_under_feet(&mut controller);

    let labels = controller.draw_list(0, None);
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].value, 7);
}

#[test]
fn teleport_drops_labels_of_the_old_area() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);

    controller.frame(FrameStatus { feet: BlockPos { x: feet().x + 1000, ..feet() }, ..in_world(2) });

    assert!(controller.draw_list(0, None).is_empty());
}

#[test]
fn many_game_tags_keep_the_nearest_labels() {
    let mut config = LevelsConfig::default();
    config.scan.max_labels = 10;
    let mut controller = turned_on_with(config);
    controller.frame(in_world(1));
    for dx in 0..20 {
        controller.submit_column(ColumnPos { x: feet().x + dx, z: feet().z }, &[sample(feet().x + dx, 63, feet().z)]);
    }

    let labels = controller.draw_list(256 + 4, None);

    assert_eq!(labels.len(), 6);
    assert!(labels.iter().all(|l| l.x < (feet().x + 6) as f32));
}

#[test]
fn submit_while_off_is_ignored() {
    let mut controller = controller();
    controller.frame(in_world(0));
    submit_under_feet(&mut controller);

    assert!(press(&mut controller));

    assert!(controller.draw_list(0, None).is_empty());
}

#[test]
fn leaving_world_turns_off_and_clears() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);

    controller.frame(out_of_world(2));
    assert!(!controller.is_on());

    controller.frame(in_world(3));
    assert!(press(&mut controller));
    assert!(controller.draw_list(0, None).is_empty());
}

#[test]
fn tick_without_a_player_keeps_the_overlay_on() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);

    let requests = controller.frame(FrameStatus { has_player: false, ..in_world(2) }).len();

    assert_eq!(requests, 0);
    assert!(controller.is_on());
    controller.frame(in_world(3));
    assert_eq!(controller.draw_list(0, None).len(), 1);
}

#[test]
fn dimension_change_clears_but_stays_on() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);

    controller.frame(FrameStatus { dimension: 1, ..in_world(2) });

    assert!(controller.is_on());
    assert!(controller.draw_list(0, None).is_empty());
}

#[test]
fn label_rotation_follows_last_yaw() {
    let mut controller = turned_on();

    controller.frame(FrameStatus { yaw: 90.0, ..in_world(1) });

    assert_eq!(controller.label_rotation(), LabelRotation::for_yaw(90.0));
}

#[test]
fn telemetry_counts_no_labels_while_off() {
    let mut controller = turned_on_with(LevelsConfig { telemetry: true, ..LevelsConfig::default() });
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);
    assert_eq!(controller.draw_list(0, None).len(), 1);
    assert!(press(&mut controller));

    run_window(&mut controller, 2);
    controller.draw_list(0, None);
    run_window(&mut controller, 130);

    assert_eq!(controller.last_report().map(|r| r.labels), Some(0));
}

#[test]
fn telemetry_reports_memory_against_the_budget() {
    let mut controller = turned_on_with(LevelsConfig { telemetry: true, ..LevelsConfig::default() });
    controller.frame(in_world(1));
    controller.timing(10.0, 20.0, 0);
    run_window(&mut controller, 2);
    let core_only = controller.last_report().map(|r| r.memory_bytes).unwrap_or(0);

    controller.timing(10.0, 20.0, 5000);
    run_window(&mut controller, 130);
    let report = controller.last_report().expect("a report after the second window");

    assert!(core_only > 0);
    assert_eq!(report.memory_bytes, core_only + 5000);
    assert_eq!(report.budget_bytes, 32 * BYTES_PER_MB);
}

#[test]
fn camera_hides_labels_behind_it() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);
    let at = [feet().x as f32 + 10.0, 66.0, feet().z as f32 + 0.5];
    let away = CameraView::from_forward(at, [1.0, 0.0, 0.0]);

    assert!(controller.draw_list(0, away).is_empty());
    assert_eq!(controller.draw_list(0, None).len(), 1);
}

#[test]
fn telemetry_reports_culled_labels_and_draws_without_a_camera() {
    let mut controller = turned_on_with(LevelsConfig { telemetry: true, ..LevelsConfig::default() });
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);
    let at = [feet().x as f32 + 10.0, 66.0, feet().z as f32 + 0.5];
    let away = CameraView::from_forward(at, [1.0, 0.0, 0.0]);

    controller.draw_list(0, None);
    controller.draw_list(0, None);
    controller.draw_list(0, away);
    run_window(&mut controller, 2);
    let report = controller.last_report().expect("a report after the window");

    assert_eq!((report.culled, report.draws_without_camera), (1, 2));
}

#[test]
fn hidden_hud_pauses_scan_and_draw_and_keeps_the_labels() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);

    let requests = controller.frame(FrameStatus { hud_hidden: true, ..in_world(2) }).len();
    let hidden = controller.draw_list(0, None).len();
    let resumed = controller.frame(in_world(3)).len();

    assert_eq!((requests, hidden), (0, 0));
    assert!(resumed > 0);
    assert_eq!(controller.draw_list(0, None).len(), 1);
    assert!(controller.is_on());
}
