use std::path::Path;

use alaydriem_levels::services::Overlay;
use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amm_core::structs::lease::AcquireOutcome;
use amm_core::structs::registry::ModIndex;
use levels_core::structs::frame::FrameStatus;
use levels_core::structs::light::SurfaceSample;
use levels_core::structs::world::{BlockPos, ColumnPos};

const OTHER_MOD: ModIndex = ModIndex(2);
const TOGGLE: ModEvent = ModEvent::Keybind { action: 0, phase: KeybindPhase::Pressed };

// an overlay registered on this thread fake manager
struct Session {
    overlay: Overlay,
}

impl Session {
    fn new() -> Self {
        Self::with_config("")
    }

    // `config` is the config.hcl text the fake manager hands out
    fn with_config(config: &str) -> Self {
        FakeTable::set_config(config);
        let overlay = Overlay::connect(FakeTable::table(), Path::new(FakeTable::CONFIG_DIR)).unwrap();
        Self { overlay }
    }

    // in world, no menu, overworld, feet at (10, 64, -20)
    fn in_world(time_seconds: f64) -> FrameStatus {
        FrameStatus {
            time_seconds,
            in_world: true,
            has_player: true,
            menu_open: false,
            hud_hidden: false,
            dimension: 0,
            feet: BlockPos { x: 10, y: 64, z: -20 },
            sky_darken: 0,
            yaw: 0.0,
        }
    }

    fn out_of_world(time_seconds: f64) -> FrameStatus {
        FrameStatus { in_world: false, has_player: false, ..Self::in_world(time_seconds) }
    }

    fn frame(&mut self, status: FrameStatus) -> usize {
        self.overlay.frame(status)
    }

    // an in-world frame, then the toggle keybind delivered on a frame without a player, so the toggle lands and the
    // scan planner is untouched until the next frame
    fn turn_on(&mut self) {
        self.frame(Self::in_world(0.0));
        FakeTable::push_event(TOGGLE);
        self.frame(FrameStatus { has_player: false, ..Self::in_world(0.0) });
    }

    fn columns_at(&mut self, frame: u32) -> usize {
        self.frame(Self::in_world(f64::from(frame) / 60.0))
    }

    fn instances(&mut self) -> usize {
        self.overlay.world_plan().map_or(0, |plan| plan.instances.len())
    }

    fn border_quads(&mut self) -> usize {
        self.overlay.world_plan().map_or(0, |plan| plan.border_quads.len())
    }

    fn under_feet(block_light: u8, sky_light: u8) -> SurfaceSample {
        SurfaceSample { pos: BlockPos { x: 10, y: 63, z: -20 }, block_light, sky_light }
    }
}

fn action(index: u16) -> ModEvent {
    ModEvent::Keybind { action: index, phase: KeybindPhase::Pressed }
}

fn acquired() -> Vec<Capability> {
    FakeTable::with(|state| state.acquired.clone())
}

fn released() -> Vec<Capability> {
    FakeTable::with(|state| state.released.clone())
}

#[test]
fn registers_without_a_frame_callback() {
    let _session = Session::new();

    assert!(FakeTable::with(|state| state.registered_without_frame()));
}

#[test]
fn toggle_keybind_turns_on_and_acquires_world_draw() {
    let mut session = Session::new();

    session.turn_on();

    assert_eq!(acquired(), [Capability::WorldDraw]);
    assert!(session.columns_at(1) > 0);
}

#[test]
fn toggle_without_world_takes_no_lease() {
    let mut session = Session::new();
    FakeTable::push_event(TOGGLE);

    session.frame(Session::out_of_world(0.0));

    assert!(acquired().is_empty());
    assert_eq!(session.columns_at(1), 0);
}

#[test]
fn toggle_with_menu_open_takes_no_lease() {
    let mut session = Session::new();
    session.frame(FrameStatus { menu_open: true, ..Session::in_world(0.0) });
    FakeTable::push_event(TOGGLE);

    session.frame(FrameStatus { menu_open: true, ..Session::in_world(1.0 / 60.0) });

    assert!(acquired().is_empty());
}

#[test]
fn denied_world_draw_keeps_overlay_off() {
    FakeTable::set_acquire_outcome(Capability::WorldDraw, AcquireOutcome::Denied { holder: OTHER_MOD });
    let mut session = Session::new();

    session.turn_on();

    assert_eq!(session.columns_at(1), 0);
    assert_eq!(session.columns_at(2), 0);
    assert_eq!(acquired(), [Capability::WorldDraw]);
}

#[test]
fn revoked_world_draw_hides_the_overlay() {
    let mut session = Session::new();
    session.turn_on();
    FakeTable::push_event(ModEvent::Revoked { capability: Capability::WorldDraw, by: OTHER_MOD });

    assert_eq!(session.columns_at(1), 0);
    assert!(released().is_empty());
}

#[test]
fn suspended_world_draw_hides_the_overlay_and_keeps_the_lease() {
    let mut session = Session::new();
    session.turn_on();
    FakeTable::push_event(ModEvent::Suspended { capability: Capability::WorldDraw, by: OTHER_MOD });

    assert_eq!(session.columns_at(1), 0);
    assert!(session.overlay.is_on());
    assert!(released().is_empty());
}

#[test]
fn suspended_lease_is_given_back_once_nothing_is_wanted() {
    let mut session = Session::new();
    session.turn_on();
    FakeTable::push_event(ModEvent::Suspended { capability: Capability::WorldDraw, by: OTHER_MOD });
    session.columns_at(1);
    FakeTable::push_event(TOGGLE);

    session.columns_at(2);

    assert_eq!(released(), [Capability::WorldDraw]);
}

#[test]
fn suspend_then_resume_keeps_everything() {
    let mut session = Session::new();
    session.turn_on();
    session.frame(Session::in_world(1.0 / 60.0));
    FakeTable::push_event(action(1));
    FakeTable::push_event(action(2));
    session.columns_at(2);
    let before = session.instances();
    assert_eq!(before, 1);

    FakeTable::push_event(ModEvent::Suspended { capability: Capability::WorldDraw, by: OTHER_MOD });
    assert_eq!(session.columns_at(3), 0);
    assert!(session.overlay.world_plan().is_none());
    FakeTable::push_event(ModEvent::Resumed(Capability::WorldDraw));

    assert!(session.columns_at(4) > 0);
    assert_eq!(session.instances(), before);
    assert!(session.border_quads() > 0);
    assert_eq!(acquired(), [Capability::WorldDraw]);
    assert!(released().is_empty());
}

#[test]
fn new_keybinds_reach_core() {
    let mut session = Session::new();
    session.frame(Session::in_world(0.0));

    FakeTable::push_event(action(1));
    session.columns_at(1);
    assert!(session.border_quads() > 0);

    FakeTable::push_event(action(2));
    session.columns_at(2);
    assert_eq!(session.instances(), 1);

    FakeTable::push_event(action(3));
    session.columns_at(3);
    assert_eq!(session.instances(), 0);

    FakeTable::push_event(action(2));
    FakeTable::push_event(action(2));
    session.columns_at(4);
    FakeTable::push_event(action(4));
    session.columns_at(5);
    assert_eq!(session.instances(), 0);
}

#[test]
fn revoke_waits_for_capability_available() {
    let mut session = Session::new();
    session.turn_on();
    session.frame(Session::in_world(1.0 / 60.0));
    FakeTable::push_event(action(2));
    session.columns_at(2);
    FakeTable::push_event(ModEvent::Revoked { capability: Capability::WorldDraw, by: OTHER_MOD });
    session.columns_at(3);
    assert!(session.overlay.world_plan().is_none());

    for frame in 4..14 {
        session.columns_at(frame);
    }
    assert_eq!(acquired().len(), 1);

    FakeTable::push_event(ModEvent::CapabilityAvailable(Capability::WorldDraw));
    session.columns_at(14);

    assert_eq!(acquired().len(), 2);
    assert_eq!(session.instances(), 1);
}

#[test]
fn disabled_turns_overlay_off() {
    let mut session = Session::new();
    session.turn_on();
    FakeTable::push_event(ModEvent::Disabled);

    assert_eq!(session.columns_at(1), 0);
    assert!(released().is_empty());
}

#[test]
fn lost_lease_keeps_the_overlay_on_but_hidden_and_does_not_retry() {
    let mut session = Session::new();
    session.turn_on();
    FakeTable::push_event(ModEvent::Revoked { capability: Capability::WorldDraw, by: OTHER_MOD });
    session.columns_at(1);

    assert_eq!(session.columns_at(2), 0);
    assert_eq!(session.columns_at(3), 0);
    assert!(session.overlay.is_on());
    assert_eq!(acquired(), [Capability::WorldDraw]);
}

#[test]
fn a_lost_lease_is_asked_for_again_after_a_world_leave() {
    FakeTable::set_acquire_outcome(Capability::WorldDraw, AcquireOutcome::Denied { holder: OTHER_MOD });
    let mut session = Session::new();
    session.turn_on();
    assert_eq!(session.columns_at(1), 0);
    session.frame(Session::out_of_world(2.0 / 60.0));
    FakeTable::set_acquire_outcome(Capability::WorldDraw, AcquireOutcome::Granted);

    session.turn_on();

    assert_eq!(acquired(), [Capability::WorldDraw, Capability::WorldDraw]);
    assert!(session.columns_at(3) > 0);
}

#[test]
fn leaving_world_releases_world_draw() {
    let mut session = Session::new();
    session.turn_on();

    session.frame(Session::out_of_world(1.0 / 60.0));

    assert_eq!(released(), [Capability::WorldDraw]);
}

#[test]
fn repeat_and_release_phases_do_not_toggle() {
    let mut session = Session::new();
    session.turn_on();
    FakeTable::push_event(ModEvent::Keybind { action: 0, phase: KeybindPhase::Repeat });
    FakeTable::push_event(ModEvent::Keybind { action: 0, phase: KeybindPhase::Released });

    assert!(session.columns_at(1) > 0);
    assert_eq!(acquired(), [Capability::WorldDraw]);
    assert!(released().is_empty());
}

#[test]
fn settings_changed_rereads_the_document() {
    let mut session = Session::new();
    session.turn_on();
    session.frame(Session::in_world(1.0 / 60.0));
    FakeTable::set_config("label {\n  yellow = [0.0, 0.0, 1.0, 1.0]\n}\n");
    FakeTable::push_event(ModEvent::SettingsChanged { paths: 1 });
    session.frame(Session::in_world(2.0 / 60.0));
    session.overlay.submit_column(ColumnPos { x: 10, z: -20 }, &[Session::under_feet(7, 0)]);

    let plan = session.overlay.world_plan().unwrap();

    assert!(!plan.label_quads.is_empty());
    assert!(plan.label_quads.iter().all(|quad| quad.rgba[..3] == [0x00, 0x00, 0xFF]), "{:?}", plan.label_quads);
}

#[test]
fn unreadable_settings_keep_the_last_config() {
    let mut session = Session::with_config("label {\n  yellow = [0.0, 0.0, 1.0, 1.0]\n}\n");
    session.turn_on();
    session.frame(Session::in_world(1.0 / 60.0));
    FakeTable::set_config("label {");
    FakeTable::push_event(ModEvent::SettingsChanged { paths: 1 });
    session.frame(Session::in_world(2.0 / 60.0));
    session.overlay.submit_column(ColumnPos { x: 10, z: -20 }, &[Session::under_feet(7, 0)]);

    let plan = session.overlay.world_plan().unwrap();

    assert!(!plan.label_quads.is_empty());
    assert!(plan.label_quads.iter().all(|quad| quad.rgba[..3] == [0x00, 0x00, 0xFF]), "{:?}", plan.label_quads);
}

#[test]
fn the_world_plan_carries_the_labels() {
    let mut session = Session::new();
    session.turn_on();
    session.frame(Session::in_world(1.0 / 60.0));
    session.overlay.submit_column(ColumnPos { x: 10, z: -20 }, &[Session::under_feet(7, 0)]);

    let plan = session.overlay.world_plan().unwrap();

    assert!(plan.label_version >= 1);
    assert!(!plan.label_quads.is_empty());
}
