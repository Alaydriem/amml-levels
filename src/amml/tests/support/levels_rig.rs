use std::collections::HashMap;

use alaydriem_levels::LevelsMod;
use amm_client::test_support::FakeTable;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amml_api::amml_abi::{ClientTick, ScreenRender, WorldRendered};
use amml_api::testing::{FakeCanvas, FakeChunk, FakeClient, FakeGame, FakeScreen, FakeWorld};
use amml_api::{
    Aabb, ActorBox, ActorTypeId, Area, BlockPos, ChunkLoad, Rotation, Screen, ScreenId, Size, SurfaceSample, Vec3,
    ViewId,
};

pub const TOGGLE: u16 = 0;
pub const BORDERS: u16 = 1;
pub const SPHERE_DROP: u16 = 2;
pub const SPHERE_CLEAR: u16 = 4;
pub const INFO: u16 = 5;

// the levels mod enabled on a fake game that serves every row of its `USES`. The player stands at (10.5, 64, -19.5) in
// the overworld with a lit-by-block surface under the feet
pub struct LevelsRig {
    pub game: FakeGame,
}

impl LevelsRig {
    pub const FEET: Vec3 = Vec3 { x: 10.5, y: 64.0, z: -19.5 };
    // a zombie: kind 0x20 with the mob and monster family bits
    pub const ZOMBIE: ActorTypeId = ActorTypeId(0x0B20);

    // every area of the `USES` of the mod, the optional `"info"` rows included
    pub const AREAS: [Area; 17] = [
        Area::INPUT_LEASE,
        Area::CLIENT_READ,
        Area::CLIENT_HUD,
        Area::ACTOR_READ.at(3),
        Area::WORLD_SKY_DARKEN,
        Area::WORLD_SCAN_SURFACES,
        Area::RENDER_WORLD_CANVAS.at(2),
        Area::UI_SCREEN,
        Area::UI_DRAW,
        Area::UI_TEXT,
        Area::CLIENT_TOP_SCREEN,
        Area::WORLD_CHUNK_READ,
        Area::WORLD_BIOMES.at(2),
        Area::WORLD_BLOCKS,
        Area::PLAYER_HIT_RESULT,
        Area::CLIENT_WORLD_INFO,
        Area::WORLD_ACTORS_IN.at(2),
    ];

    // the mod manager is provided; `config` is the text of its config.hcl
    pub fn with_manager(config: &str) -> Self {
        Self::build(true, config)
    }

    // the mod manager is not installed
    pub fn without_manager() -> Self {
        Self::build(false, "")
    }

    fn build(manager: bool, config: &str) -> Self {
        let game = FakeTable::game();
        game.offer::<ClientTick>();
        game.offer::<WorldRendered>();
        game.offer::<ScreenRender>();
        for area in Self::AREAS {
            game.offer_area(area);
        }
        if manager {
            FakeTable::provide(&game.ctx()).unwrap();
        }
        FakeTable::set_config(config);
        game.load::<LevelsMod>().unwrap();
        game.enable::<LevelsMod>().unwrap();
        Self { game }
    }

    // a player in the overworld at `FEET`, with surfaces under the feet and a loaded chunk square around it
    pub fn in_world() -> FakeClient {
        FakeClient {
            local_player: true,
            world_active: true,
            position: Some(Self::FEET),
            rotation: Some(Rotation { yaw: 0.0, pitch: 0.0 }),
            dimension: Some(0),
            sky_darken: Some(0),
            world: Some(Self::world()),
            ..FakeClient::default()
        }
    }

    // loaded chunks from -4 to 4 in each direction, and one surface under the feet with block light 7
    pub fn world() -> FakeWorld {
        let mut chunks = HashMap::new();
        for x in -4..=4 {
            for z in -4..=4 {
                chunks.insert((x, z), FakeChunk::new(ChunkLoad::Loaded));
            }
        }
        let under_feet = SurfaceSample { pos: BlockPos { x: 10, y: 63, z: -20 }, block_light: 7, sky_light: 0 };
        FakeWorld { chunks, surfaces: HashMap::from([((10, -20), vec![under_feet])]), ..FakeWorld::default() }
    }

    // the world of `world` with three zombies near the feet
    pub fn zombie_world() -> FakeWorld {
        FakeWorld {
            actors: vec![Self::zombie(12.0, -18.0), Self::zombie(8.0, -22.0), Self::zombie(14.0, -25.0)],
            actor_types: vec![Self::ZOMBIE; 3],
            ..Self::world()
        }
    }

    fn zombie(x: f32, z: f32) -> ActorBox {
        ActorBox {
            aabb: Aabb { min: Vec3 { x, y: 64.0, z }, max: Vec3 { x: x + 0.6, y: 65.9, z: z + 0.6 } },
            is_local_player: false,
        }
    }

    pub fn set_client(&self, client: FakeClient) {
        self.game.set_client(client);
    }

    pub fn tick(&self) {
        self.game.raise_with_client(&mut ClientTick { header: FakeGame::header::<ClientTick>() });
    }

    // a key of the descriptor pressed, then one tick
    pub fn press(&self, action: u16) {
        FakeTable::push_event(ModEvent::Keybind { action, phase: KeybindPhase::Pressed });
        self.tick();
    }

    // a key of the descriptor released, then one tick
    pub fn release(&self, action: u16) {
        FakeTable::push_event(ModEvent::Keybind { action, phase: KeybindPhase::Released });
        self.tick();
    }

    // the mod linked, the overlay on and one scan done
    pub fn turned_on() -> Self {
        let rig = Self::with_manager("");
        rig.set_client(Self::in_world());
        rig.tick();
        rig.press(TOGGLE);
        rig
    }

    // one `WorldRendered` for `canvas`
    pub fn render(&self, canvas: &FakeCanvas) {
        self.game.raise_world_rendered(canvas);
    }

    // one `WorldRendered` with the camera at the player, in the normal graphics mode
    pub fn frame(&self) -> FakeCanvas {
        let canvas = FakeCanvas { camera_position: Some(Self::FEET), ..FakeCanvas::default() };
        self.render(&canvas);
        canvas
    }

    // `in_world` with the HUD on top and no menu open
    pub fn on_hud() -> FakeClient {
        FakeClient { top_screen: Screen::Hud, ..Self::in_world() }
    }

    // one `ScreenRender` view of the size the game drew in the probe run; the ui draws it made
    pub fn view(&self) -> Vec<String> {
        let screen = FakeScreen { size: Some(Size { width: 719, height: 426 }), ..FakeScreen::default() };
        let before = self.game.draws().len();
        self.game.raise_screen_render(ScreenId(9), ViewId(10), &screen);
        self.game.draws().split_off(before)
    }

    // the lines the mod logged that start with `prefix`
    pub fn lines(&self, prefix: &str) -> Vec<String> {
        self.game.logs().into_iter().filter(|line| line.starts_with(prefix)).collect()
    }

    pub fn acquired() -> Vec<amm_core::structs::descriptor::Capability> {
        FakeTable::with(|state| state.acquired.clone())
    }

    pub fn released() -> Vec<amm_core::structs::descriptor::Capability> {
        FakeTable::with(|state| state.released.clone())
    }
}
