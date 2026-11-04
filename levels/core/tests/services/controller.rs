use levels_core::config::{LevelsConfig, ScanConfig};
use levels_core::services::LevelsController;
use levels_core::structs::frame::FrameStatus;
use levels_core::structs::light::SurfaceSample;
use levels_core::structs::manager::ManagerSignal;
use levels_core::structs::world::BlockPos;

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

// the quads of the label mesh in this frame's plan; zero when the plan has none
fn label_quads(controller: &mut LevelsController) -> usize {
    controller.world_plan().map_or(0, |plan| plan.label_quads.len())
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
    assert!(controller.world_plan().is_none());

    controller.signal(ManagerSignal::LeaseRestored);
    controller.frame(in_world(3));

    assert!(label_quads(&mut controller) > 0);
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

    let plan = controller.world_plan().expect("labels on");
    assert!(!plan.label_quads.is_empty());
    assert!(plan.label_quads.iter().all(|quad| quad.rgba == [0, 0, 255, 255]));
}

#[test]
fn apply_config_sanitizes_light_bands() {
    let mut controller = turned_on();
    submit_under_feet(&mut controller);
    let mut config = LevelsConfig::default();
    config.light.red_max = 9;
    config.light.yellow_max = 3;

    controller.apply_config(config);

    let red = LevelsConfig::default().label.red.map(|c| (c * 255.0).round() as u8);
    let plan = controller.world_plan().expect("labels on");
    assert!(!plan.label_quads.is_empty());
    assert!(plan.label_quads.iter().all(|quad| quad.rgba == red));
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
    assert!(controller.world_plan().is_none());
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

    assert!(label_quads(&mut controller) > 0);
}

#[test]
fn teleport_drops_labels_of_the_old_area() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);

    controller.frame(FrameStatus { feet: BlockPos { x: feet().x + 1000, ..feet() }, ..in_world(2) });

    assert_eq!(label_quads(&mut controller), 0);
}

#[test]
fn submit_while_off_is_ignored() {
    let mut controller = controller();
    controller.frame(in_world(0));
    submit_under_feet(&mut controller);

    assert!(press(&mut controller));

    assert_eq!(label_quads(&mut controller), 0);
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
    assert_eq!(label_quads(&mut controller), 0);
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
    assert!(label_quads(&mut controller) > 0);
}

#[test]
fn dimension_change_clears_but_stays_on() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);

    controller.frame(FrameStatus { dimension: 1, ..in_world(2) });

    assert!(controller.is_on());
    assert_eq!(label_quads(&mut controller), 0);
}

#[test]
fn telemetry_counts_no_labels_while_off() {
    let mut controller = turned_on_with(LevelsConfig { telemetry: true, ..LevelsConfig::default() });
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);
    assert!(label_quads(&mut controller) > 0);
    assert!(press(&mut controller));

    run_window(&mut controller, 2);
    controller.world_plan();
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
fn hidden_hud_pauses_scan_and_draw_and_keeps_the_labels() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);

    let requests = controller.frame(FrameStatus { hud_hidden: true, ..in_world(2) }).len();
    let hidden = label_quads(&mut controller);
    let resumed = controller.frame(in_world(3)).len();

    assert_eq!((requests, hidden), (0, 0));
    assert!(resumed > 0);
    assert!(label_quads(&mut controller) > 0);
    assert!(controller.is_on());
}

#[test]
fn world_plan_has_label_quads_when_on() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);

    let plan = controller.world_plan().expect("labels on");

    assert!(plan.label_version > 0);
    assert!(!plan.label_quads.is_empty());
    assert!(plan.border_quads.is_empty() && plan.instances.is_empty());
}

#[test]
fn world_plan_label_version_is_stable_without_change() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);
    let first = controller.world_plan().expect("labels on").label_version;

    let same = controller.world_plan().expect("labels on").label_version;
    controller.frame(FrameStatus { yaw: 180.0, ..in_world(2) });
    let turned = controller.world_plan().expect("labels on").label_version;

    assert_eq!(first, same);
    assert_ne!(first, turned);
}

#[test]
fn hidden_hud_gives_no_label_quads() {
    let mut controller = turned_on();
    controller.frame(in_world(1));
    submit_under_feet(&mut controller);
    assert!(label_quads(&mut controller) > 0);

    controller.frame(FrameStatus { hud_hidden: true, ..in_world(2) });

    let plan = controller.world_plan().expect("labels still on");
    assert_eq!((plan.label_version, plan.label_quads.len()), (0, 0));
}

#[test]
fn world_plan_is_none_when_off_and_no_markers() {
    let mut controller = controller();
    controller.frame(in_world(0));

    assert!(controller.world_plan().is_none());
}

#[test]
fn markers_alone_give_a_plan_without_label_quads() {
    let mut controller = controller();
    controller.frame(in_world(0));
    controller.signal(ManagerSignal::BordersPressed);
    controller.frame(in_world(1));

    let plan = controller.world_plan().expect("borders on");

    assert_eq!((plan.label_version, plan.label_quads.len()), (0, 0));
    assert!(!plan.border_quads.is_empty());
}
