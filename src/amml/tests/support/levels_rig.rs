use std::collections::HashMap;

use alaydriem_levels::LevelsMod;
use amm_client::test_support::FakeTable;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amml_api::amml_abi::{ClientTick, WorldRendered};
use amml_api::testing::{FakeCanvas, FakeChunk, FakeClient, FakeGame, FakeWorld};
use amml_api::{Area, BlockPos, ChunkLoad, Rotation, SurfaceSample, Vec3};

pub const TOGGLE: u16 = 0;
pub const BORDERS: u16 = 1;
pub const SPHERE_DROP: u16 = 2;
pub const SPHERE_CLEAR: u16 = 4;

// the levels mod enabled on a fake game that serves every row of its `USES`. The player stands at (10.5, 64, -19.5) in
// the overworld with a lit-by-block surface under the feet
pub struct LevelsRig {
    pub game: FakeGame,
}

impl LevelsRig {
    pub const FEET: Vec3 = Vec3 { x: 10.5, y: 64.0, z: -19.5 };

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
        for area in [
            Area::INPUT_LEASE,
            Area::CLIENT_READ,
            Area::CLIENT_HUD,
            Area::ACTOR_READ.at(3),
            Area::WORLD_SKY_DARKEN,
            Area::WORLD_SCAN_SURFACES,
            Area::RENDER_WORLD_CANVAS.at(2),
        ] {
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
