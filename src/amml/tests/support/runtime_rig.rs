use std::sync::{Arc, Mutex};

use alaydriem_levels::services::{LevelsLog, LevelsRuntime, ManagerSession};
use alaydriem_levels::types::LinkStage;
use amm_client::test_support::FakeTable;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amml_api::testing::{FakeClient, FakeGame};

use crate::support::levels_rig::LevelsRig;
use crate::support::probe::Probe;

// a runtime on a fake game that serves every area of the mod, linked to the manager by one tick. Each tick serves the
// client of `set_client`, at first the player of `LevelsRig::in_world`
pub struct RuntimeRig {
    pub game: FakeGame,
    pub runtime: Arc<LevelsRuntime>,
    client: Mutex<FakeClient>,
}

impl RuntimeRig {
    // `config` is the text of the manager's config.hcl
    pub fn new(config: &str) -> Self {
        let game = FakeTable::game();
        for area in LevelsRig::AREAS {
            game.offer_area(area);
        }
        FakeTable::provide(&game.ctx()).unwrap();
        FakeTable::set_config(config);
        let ctx = FakeTable::ctx_of(&game, ManagerSession::DESCRIPTOR);
        let runtime = Arc::new(LevelsRuntime::new(LevelsLog::new(ctx.log()), ctx));
        runtime.attach(ManagerSession::start(&ctx).unwrap());
        let rig = Self { game, runtime, client: Mutex::new(LevelsRig::in_world()) };
        rig.tick();
        rig
    }

    pub fn set_client(&self, client: FakeClient) {
        *self.client.lock().unwrap() = client;
    }

    pub fn tick(&self) {
        let runtime = Arc::clone(&self.runtime);
        let client = self.client.lock().unwrap().clone();
        Probe::read(&self.game, client, move |client| runtime.tick(client));
    }

    // one manager event, then one tick
    pub fn push(&self, event: ModEvent) {
        FakeTable::push_event(event);
        self.tick();
    }

    pub fn press(&self, action: u16) {
        self.push(ModEvent::Keybind { action, phase: KeybindPhase::Pressed });
    }

    pub fn release(&self, action: u16) {
        self.push(ModEvent::Keybind { action, phase: KeybindPhase::Released });
    }

    // whether core shows the info panel; false before the overlay runs
    pub fn info_shown(&self) -> bool {
        self.runtime.inspect(|state| match &state.stage {
            LinkStage::Running(overlay) => overlay.controller().info().is_shown(),
            _ => false,
        })
    }

    // the text of the lines of the info panel; empty before the overlay runs
    pub fn info_lines(&self) -> Vec<String> {
        self.runtime.inspect(|state| match &state.stage {
            LinkStage::Running(overlay) => {
                overlay.controller().info().lines().iter().map(|line| line.as_str().to_owned()).collect()
            }
            _ => Vec::new(),
        })
    }
}
