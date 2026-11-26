use std::sync::{Arc, Mutex};

use alaydriem_levels::services::InfoDrawer;
use amm_client::test_support::FakeTable;
use amml_api::amml_abi::ScreenRender;
use amml_api::testing::{FakeGame, FakeScreen};
use amml_api::{Area, HookOrder, Screen, ScreenId, Size, ViewId};
use levels_core::config::LevelsConfig;
use levels_core::services::InfoPanel;
use levels_core::structs::info::{InfoNames, InfoSample};
use levels_core::structs::manager::ManagerSignal;
use levels_core::structs::world::Vec3f;

// an info drawer and a panel of the default config filled by one refresh, on a fake game that serves the ui areas
struct Rig {
    game: FakeGame,
    drawer: Arc<Mutex<InfoDrawer>>,
    panel: Arc<Mutex<InfoPanel>>,
}

impl Rig {
    fn new(shown: bool) -> Self {
        let game = FakeTable::game();
        game.offer::<ScreenRender>();
        for area in [Area::UI_SCREEN, Area::UI_DRAW, Area::UI_TEXT] {
            game.offer_area(area);
        }
        let config = LevelsConfig::default();
        let mut panel = InfoPanel::new(&config.info, config.light, &config.label);
        panel.signal(ManagerSignal::InfoPressed);
        let sample = InfoSample { position: Vec3f { x: 10.5, y: 64.0, z: -19.5 }, ..InfoSample::default() };
        panel.refresh(&sample, InfoNames::default(), &[]);
        if !shown {
            panel.signal(ManagerSignal::InfoPressed);
        }
        Self { game, drawer: Arc::new(Mutex::new(InfoDrawer::new())), panel: Arc::new(Mutex::new(panel)) }
    }

    fn screen(height: u32) -> FakeScreen {
        FakeScreen { size: Some(Size { width: 719, height }), ..FakeScreen::default() }
    }

    // one `ScreenRender` view handed to the drawer; the draws it made
    fn view(&self, top_screen: Option<Screen>, game_thread: bool, frame: u64) -> Vec<String> {
        self.view_sized(top_screen, game_thread, frame, 426)
    }

    fn view_sized(&self, top_screen: Option<Screen>, game_thread: bool, frame: u64, height: u32) -> Vec<String> {
        let drawer = Arc::clone(&self.drawer);
        let panel = Arc::clone(&self.panel);
        let before = self.game.draws().len();
        let listener = self
            .game
            .ctx()
            .events()
            .listen::<ScreenRender>(HookOrder::NORMAL, move |event, _| {
                drawer.lock().unwrap().draw(event, top_screen, game_thread, &panel.lock().unwrap(), frame);
            })
            .unwrap();
        self.game.raise_screen_render(ScreenId(9), ViewId(10), &Self::screen(height));
        drop(listener);
        self.game.draws().split_off(before)
    }

    fn line_count(&self) -> usize {
        self.panel.lock().unwrap().lines().len()
    }
}

#[test]
fn draws_background_and_lines_on_hud() {
    let rig = Rig::new(true);

    let draws = rig.view(Some(Screen::Hud), true, 1);

    assert!(rig.line_count() > 1);
    assert_eq!(draws.len(), 1 + rig.line_count(), "{draws:?}");
    assert!(draws[0].starts_with("fill "), "{draws:?}");
    assert!(draws[1..].iter().all(|draw| draw.starts_with("text ")), "{draws:?}");
    assert!(draws[1].starts_with("text \"XYZ: 10.500"), "{draws:?}");
}

#[test]
fn no_draw_on_menu() {
    let rig = Rig::new(true);

    assert!(rig.view(Some(Screen::Pause), true, 1).is_empty());
    assert!(rig.view(None, true, 2).is_empty());
}

#[test]
fn no_draw_when_hidden() {
    let rig = Rig::new(false);

    assert!(rig.view(Some(Screen::Hud), true, 1).is_empty());
}

#[test]
fn off_game_thread_draws_nothing() {
    let rig = Rig::new(true);

    assert!(rig.view(Some(Screen::Hud), false, 1).is_empty());
}

#[test]
fn second_view_in_same_frame_draws_nothing() {
    let rig = Rig::new(true);

    assert!(!rig.view(Some(Screen::Hud), true, 1).is_empty());
    assert!(rig.view(Some(Screen::Hud), true, 1).is_empty());

    assert!(!rig.view(Some(Screen::Hud), true, 2).is_empty());
}

#[test]
fn missing_ui_text_draws_nothing_and_does_not_panic() {
    let rig = Rig::new(true);
    rig.game.withdraw_area(Area::UI_TEXT);

    assert!(rig.view(Some(Screen::Hud), true, 1).is_empty());
}

#[test]
fn no_lines_draws_nothing() {
    let rig = Rig::new(true);
    rig.panel.lock().unwrap().place_changed();

    assert_eq!(rig.line_count(), 0);
    assert!(rig.view(Some(Screen::Hud), true, 1).is_empty());
}

#[test]
fn no_visible_line_draws_nothing() {
    let rig = Rig::new(true);

    assert!(rig.view_sized(Some(Screen::Hud), true, 1, 10).is_empty());
}
