use levels_core::config::{BordersConfig, SpheresConfig};
use levels_core::services::{SphereShellBuilder, WorldMarkers};
use levels_core::structs::frame::FrameStatus;
use levels_core::structs::manager::ManagerSignal;
use levels_core::structs::world::BlockPos;

use crate::support::frames::{feet, in_world, out_of_world};

fn markers() -> WorldMarkers {
    WorldMarkers::new(BordersConfig::default(), SpheresConfig::default())
}

fn in_world_markers() -> WorldMarkers {
    let mut markers = markers();
    markers.frame(&in_world(0));
    markers
}

fn press(markers: &mut WorldMarkers, signal: ManagerSignal, status: FrameStatus) {
    markers.signal(signal, Some(status));
}

fn instances(markers: &WorldMarkers) -> usize {
    markers.plan().map_or(0, |plan| plan.instances.len())
}

fn at(x_offset: i32, time: u32) -> FrameStatus {
    FrameStatus { feet: BlockPos { x: feet().x + x_offset, ..feet() }, ..in_world(time) }
}

#[test]
fn cycle_and_drop_need_a_world() {
    let mut markers = markers();

    markers.signal(ManagerSignal::BordersPressed, None);
    markers.signal(ManagerSignal::SphereDropPressed, None);
    markers.signal(ManagerSignal::BordersPressed, Some(out_of_world(0)));

    assert!(markers.plan().is_none());
    assert!(!markers.is_active());
}

#[test]
fn input_is_ignored_with_a_menu_open() {
    let mut markers = markers();
    let menu = FrameStatus { menu_open: true, ..in_world(0) };
    markers.frame(&menu);

    press(&mut markers, ManagerSignal::BordersPressed, menu);
    press(&mut markers, ManagerSignal::SphereDropPressed, menu);

    assert!(!markers.is_active());
}

#[test]
fn drop_then_plan_has_one_instance_per_marker() {
    let mut markers = in_world_markers();

    press(&mut markers, ManagerSignal::SphereDropPressed, in_world(0));
    markers.frame(&in_world(1));

    let plan = markers.plan().expect("a plan in the world");
    assert_eq!(plan.instances.len(), 1);
}

#[test]
fn shell_quads_hold_the_inner_shell_then_the_outer() {
    let mut markers = in_world_markers();
    press(&mut markers, ManagerSignal::SphereDropPressed, in_world(0));
    markers.frame(&in_world(1));

    let plan = markers.plan().expect("a plan");
    let spheres = SpheresConfig::default();
    let split = SphereShellBuilder::quad_count();
    assert_eq!(plan.shell_quads.len(), SphereShellBuilder::pair_quad_count());
    let distance = |quad: &levels_core::structs::draw::ColorQuad| {
        let [x, y, z] = quad.corners[0];
        (x * x + y * y + z * z).sqrt()
    };
    for quad in &plan.shell_quads[..split] {
        assert!((distance(quad) - spheres.inner_radius).abs() < 0.5);
    }
    for quad in &plan.shell_quads[split..] {
        assert!((distance(quad) - spheres.outer_radius).abs() < 0.5);
    }
}

#[test]
fn instances_follow_show_distance() {
    let mut markers = in_world_markers();
    press(&mut markers, ManagerSignal::SphereDropPressed, in_world(0));

    markers.frame(&at(97, 1));
    assert_eq!(instances(&markers), 0);

    markers.frame(&at(96, 2));
    assert_eq!(instances(&markers), 1);
}

#[test]
fn remove_takes_the_nearest_and_clear_takes_all() {
    let mut markers = in_world_markers();
    press(&mut markers, ManagerSignal::SphereDropPressed, in_world(0));
    press(&mut markers, ManagerSignal::SphereDropPressed, at(10, 0));
    press(&mut markers, ManagerSignal::SphereRemovePressed, at(9, 0));
    markers.frame(&at(9, 1));
    assert_eq!(instances(&markers), 1);
    let remaining = markers.plan().expect("a plan").instances[0].center[0];
    assert_eq!(remaining, feet().x as f32 + 0.5);

    press(&mut markers, ManagerSignal::SphereClearPressed, in_world(1));
    markers.frame(&in_world(2));

    assert_eq!(instances(&markers), 0);
    assert!(!markers.is_active());
}

#[test]
fn spin_advances_with_time() {
    let mut markers = in_world_markers();
    press(&mut markers, ManagerSignal::SphereDropPressed, in_world(0));
    let spin = |markers: &WorldMarkers| markers.plan().expect("a plan").instances[0].spin_degrees;

    markers.frame(&FrameStatus { time_seconds: 2.0, ..in_world(0) });
    assert_eq!(spin(&markers), 5.0);

    markers.frame(&FrameStatus { time_seconds: 150.0, ..in_world(0) });
    assert_eq!(spin(&markers), 15.0);
}

#[test]
fn suspend_hides_and_keeps_state() {
    let mut markers = in_world_markers();
    press(&mut markers, ManagerSignal::BordersPressed, in_world(0));
    press(&mut markers, ManagerSignal::BordersPressed, in_world(0));
    press(&mut markers, ManagerSignal::SphereDropPressed, in_world(0));
    markers.frame(&in_world(1));
    let version = markers.plan().expect("a plan").border_version;

    markers.signal(ManagerSignal::LeaseSuspended, Some(in_world(1)));
    markers.frame(&in_world(2));
    assert!(markers.plan().is_none());
    assert!(markers.is_active());

    markers.signal(ManagerSignal::LeaseRestored, Some(in_world(2)));
    let plan = markers.plan().expect("a plan after the lease is back");
    assert_eq!(plan.border_version, version);
    assert_eq!(plan.instances.len(), 1);
}

#[test]
fn lease_lost_hides_like_suspended() {
    let mut markers = in_world_markers();
    press(&mut markers, ManagerSignal::SphereDropPressed, in_world(0));

    markers.signal(ManagerSignal::LeaseLost, Some(in_world(0)));

    assert!(markers.plan().is_none());
    assert!(markers.is_active());
}

#[test]
fn disable_and_world_leave_clear() {
    let mut disabled = in_world_markers();
    press(&mut disabled, ManagerSignal::BordersPressed, in_world(0));
    press(&mut disabled, ManagerSignal::SphereDropPressed, in_world(0));
    disabled.signal(ManagerSignal::LeaseSuspended, Some(in_world(0)));
    disabled.signal(ManagerSignal::Disabled, Some(in_world(0)));
    disabled.frame(&in_world(1));
    assert!(!disabled.is_active());
    assert!(disabled.plan().is_none());
    press(&mut disabled, ManagerSignal::BordersPressed, in_world(1));
    disabled.frame(&in_world(2));
    assert!(disabled.plan().is_some(), "the lease hiding is cleared");

    let mut left = in_world_markers();
    press(&mut left, ManagerSignal::BordersPressed, in_world(0));
    press(&mut left, ManagerSignal::SphereDropPressed, in_world(0));
    left.frame(&out_of_world(1));
    assert!(!left.is_active());
    assert!(left.plan().is_none());
}

#[test]
fn single_mode_plans_border_quads_for_the_player_chunk() {
    let mut markers = in_world_markers();

    press(&mut markers, ManagerSignal::BordersPressed, in_world(0));
    markers.frame(&in_world(1));

    let plan = markers.plan().expect("a plan");
    assert!(!plan.border_quads.is_empty());
    assert_eq!((plan.border_origin.x, plan.border_origin.z), (8, -24));
}

#[test]
fn plan_is_none_while_inactive() {
    let mut markers = in_world_markers();
    markers.frame(&in_world(1));
    assert!(!markers.is_active());
    assert!(markers.plan().is_none());

    press(&mut markers, ManagerSignal::SphereDropPressed, in_world(1));
    markers.frame(&in_world(2));
    assert!(markers.plan().is_some());

    press(&mut markers, ManagerSignal::SphereClearPressed, in_world(2));
    markers.frame(&in_world(3));
    assert!(markers.plan().is_none());
}

#[test]
fn shell_rebuilt_once_per_config_change() {
    let mut markers = in_world_markers();
    press(&mut markers, ManagerSignal::BordersPressed, in_world(0));
    let first = markers.plan().expect("a plan").shell_version;
    for i in 1..=10 {
        markers.frame(&in_world(i));
    }
    assert_eq!(markers.plan().expect("a plan").shell_version, first);

    let spheres = SpheresConfig { inner_color: [0.1, 0.2, 0.3, 0.4], ..SpheresConfig::default() };
    markers.apply_config(BordersConfig::default(), spheres);
    for i in 11..=20 {
        markers.frame(&in_world(i));
    }

    assert_eq!(markers.plan().expect("a plan").shell_version, first + 1);
}

#[test]
fn instances_buffer_is_bounded() {
    let spheres = SpheresConfig { max_markers: 64, ..SpheresConfig::default() };
    let mut markers = WorldMarkers::new(BordersConfig::default(), spheres);
    markers.frame(&in_world(0));
    let before = markers.resident_bytes();

    for _ in 0..80 {
        press(&mut markers, ManagerSignal::SphereDropPressed, in_world(0));
    }
    for i in 1..=100 {
        markers.frame(&in_world(i));
    }

    assert_eq!(instances(&markers), 64);
    assert_eq!(markers.resident_bytes(), before);
}
