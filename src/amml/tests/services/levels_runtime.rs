use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use alaydriem_levels::services::{LevelsLog, LevelsRuntime, ManagerSession};
use amm_client::test_support::FakeTable;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amml_api::amml_abi::WorldRendered;
use amml_api::testing::{FakeCanvas, FakeGame};
use amml_api::{Area, HookOrder, WorldRenderedExt};

use crate::support::levels_rig::LevelsRig;
use crate::support::probe::Probe;

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
        let runtime = Arc::new(LevelsRuntime::new(LevelsLog::new(ctx.log())));
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
