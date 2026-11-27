use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use alaydriem_levels::services::{LevelsLog, LevelsRuntime, ManagerSession};
use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amm_core::structs::registry::ModIndex;
use amml_api::amml_abi::WorldRendered;
use amml_api::testing::{FakeCanvas, FakeClient, FakeGame, FakeWorld};
use amml_api::{Area, BlockPos, HookOrder, WorldRenderedExt};

use crate::support::levels_rig::{INFO, LevelsRig};
use crate::support::probe::Probe;
use crate::support::runtime_rig::RuntimeRig;

// a runtime on a fake game, linked and turned on, with one frame drawn
struct Rig {
    game: FakeGame,
    runtime: Arc<LevelsRuntime>,
}

impl Rig {
    fn new() -> Self {
        let game = FakeTable::game();
        game.offer::<WorldRendered>();
        for area in [
            Area::CLIENT_READ,
            Area::CLIENT_HUD,
            Area::ACTOR_READ.at(3),
            Area::WORLD_SKY_DARKEN,
            Area::WORLD_SCAN_SURFACES,
            Area::RENDER_WORLD_CANVAS.at(2),
        ] {
            game.offer_area(area);
        }
        FakeTable::provide(&game.ctx()).unwrap();
        FakeTable::set_config("");
        let ctx = FakeTable::ctx_of(&game, ManagerSession::DESCRIPTOR);
        let runtime = Arc::new(LevelsRuntime::new(LevelsLog::new(ctx.log()), ctx));
        runtime.attach(ManagerSession::start(&ctx).unwrap());
        let rig = Self { game, runtime };
        rig.tick();
        FakeTable::push_event(ModEvent::Keybind { action: 0, phase: KeybindPhase::Pressed });
        rig.tick();
        rig
    }

    fn tick(&self) {
        let runtime = Arc::clone(&self.runtime);
        Probe::read(&self.game, LevelsRig::in_world(), move |client| runtime.tick(client));
    }

    // one `WorldRendered` handed to the runtime; the calls of the frame
    fn render(&self) -> Vec<String> {
        let runtime = Arc::clone(&self.runtime);
        let canvas = FakeCanvas { camera_position: Some(LevelsRig::FEET), ..FakeCanvas::default() };
        let listener = self
            .game
            .ctx()
            .events()
            .listen::<WorldRendered>(HookOrder::NORMAL, move |event, _| {
                assert!(event.canvas().is_some());
                runtime.render(event);
            })
            .unwrap();
        self.game.raise_world_rendered(&canvas);
        drop(listener);
        canvas.calls.borrow().clone()
    }
}

#[test]
fn a_panic_in_an_earlier_call_does_not_stop_the_draws() {
    let rig = Rig::new();
    assert!(!rig.render().is_empty());

    let poisoned = catch_unwind(AssertUnwindSafe(|| rig.runtime.inspect(|_| panic!("a panic under the lock"))));
    assert!(poisoned.is_err());

    assert!(rig.render().iter().any(|call| call.starts_with("draw")));
}

#[test]
fn hold_release_hides() {
    let rig = RuntimeRig::new("info {\n  mode = \"hold\"\n}\n");

    rig.press(INFO);
    assert!(rig.info_shown());
    rig.release(INFO);

    assert!(!rig.info_shown());
    assert_eq!(LevelsRig::released(), [Capability::Hud]);
}

#[test]
fn a_hud_lease_granted_again_after_a_revoke_shows_the_panel() {
    let rig = RuntimeRig::new("");
    rig.press(INFO);
    assert!(rig.info_shown());

    rig.push(ModEvent::Revoked { capability: Capability::Hud, by: ModIndex(2) });
    assert!(!rig.info_shown());
    rig.press(INFO);
    rig.press(INFO);

    assert!(rig.info_shown());
    assert_eq!(LevelsRig::acquired(), [Capability::Hud, Capability::Hud]);
}

#[test]
fn a_suspended_hud_lease_hides_the_panel_until_it_resumes() {
    let rig = RuntimeRig::new("");
    rig.press(INFO);

    rig.push(ModEvent::Suspended { capability: Capability::Hud, by: ModIndex(2) });
    assert!(!rig.info_shown());
    rig.push(ModEvent::Resumed(Capability::Hud));

    assert!(rig.info_shown());
    assert_eq!(LevelsRig::acquired(), [Capability::Hud]);
    assert!(LevelsRig::released().is_empty());
}

#[test]
fn the_info_key_fills_the_panel_from_the_game_in_the_same_tick() {
    let rig = RuntimeRig::new("");
    let feet = BlockPos { x: 10, y: 64, z: -20 };
    let world = FakeWorld {
        biomes_at: HashMap::from([(feet, 7)]),
        biome_names: HashMap::from([(7, "deep_dark".to_owned())]),
        ..LevelsRig::world()
    };
    rig.set_client(FakeClient { world: Some(world), ..LevelsRig::in_world() });

    rig.press(INFO);

    let lines = rig.info_lines();
    assert!(lines.iter().any(|line| line.contains("XYZ: 10.500")), "{lines:?}");
    assert!(lines.iter().any(|line| line.contains("Biome: deep_dark")), "{lines:?}");
}

#[test]
fn the_shown_panel_is_drawn_on_the_first_hud_view_of_each_frame() {
    let rig = LevelsRig::with_manager("");
    rig.set_client(LevelsRig::on_hud());
    rig.tick();
    rig.press(INFO);

    let first = rig.view();
    assert!(first.first().is_some_and(|draw| draw.starts_with("fill ")), "{first:?}");
    assert!(first.iter().any(|draw| draw.starts_with("text \"XYZ: ")), "{first:?}");
    assert!(rig.view().is_empty());
    rig.frame();

    assert_eq!(rig.view(), first);
}

#[test]
fn missing_actor_v2_hides_entity_lines_only() {
    let rig = LevelsRig::with_manager("");
    rig.game.withdraw_area(Area::WORLD_ACTORS_IN);
    rig.game.offer_area(Area::WORLD_ACTORS_IN);
    rig.set_client(FakeClient { world: Some(LevelsRig::zombie_world()), ..LevelsRig::on_hud() });
    rig.tick();
    rig.press(INFO);

    let draws = rig.view();

    assert!(draws.iter().any(|draw| draw.starts_with("text \"XYZ: ")), "{draws:?}");
    assert!(!draws.iter().any(|draw| draw.contains("Entities:")), "{draws:?}");
}

#[test]
fn actor_v2_withdrawn_after_a_scan_hides_the_entity_lines() {
    let rig = LevelsRig::with_manager("");
    rig.set_client(FakeClient { world: Some(LevelsRig::zombie_world()), ..LevelsRig::on_hud() });
    rig.tick();
    rig.press(INFO);
    let first = rig.view();
    assert!(first.iter().any(|draw| draw.contains("Entities: 3")), "{first:?}");

    rig.game.withdraw_area(Area::WORLD_ACTORS_IN);
    rig.game.offer_area(Area::WORLD_ACTORS_IN);
    // hiding and showing again makes the next tick scan and refresh at once
    rig.press(INFO);
    rig.press(INFO);
    rig.frame();
    let draws = rig.view();

    assert!(draws.iter().any(|draw| draw.starts_with("text \"XYZ: ")), "{draws:?}");
    assert!(!draws.iter().any(|draw| draw.contains("Entities:")), "{draws:?}");
}

#[test]
fn fps_counts_frames_skipped_by_lock() {
    let rig = RuntimeRig::new("");
    rig.game.offer::<WorldRendered>();
    let runtime = Arc::clone(&rig.runtime);
    let listener = rig
        .game
        .ctx()
        .events()
        .listen::<WorldRendered>(HookOrder::NORMAL, move |event, _| {
            runtime.render(event);
            runtime.inspect(|_| runtime.render(event));
        })
        .unwrap();
    rig.game.raise_world_rendered(&FakeCanvas { camera_position: Some(LevelsRig::FEET), ..FakeCanvas::default() });
    drop(listener);

    assert_eq!(rig.runtime.frames(), 2);
}

#[test]
fn proof_lines() {
    let rig = LevelsRig::with_manager("telemetry = true\n");
    rig.set_client(FakeClient { world: Some(LevelsRig::zombie_world()), ..LevelsRig::on_hud() });
    rig.tick();

    rig.press(INFO);
    rig.press(INFO);

    assert_eq!(
        rig.lines("info levels: info"),
        [
            "info levels: info shown mode=toggle",
            "info levels: info entities total=3 hostile=3 passive=0",
            "info levels: info hidden"
        ]
    );
}

#[test]
fn without_telemetry_no_entity_line_is_logged() {
    let rig = LevelsRig::with_manager("");
    rig.set_client(FakeClient { world: Some(LevelsRig::zombie_world()), ..LevelsRig::on_hud() });
    rig.tick();

    rig.press(INFO);

    assert_eq!(rig.lines("info levels: info"), ["info levels: info shown mode=toggle"]);
}
