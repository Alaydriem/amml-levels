use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use alaydriem_levels::services::InfoReader;
use amm_client::test_support::FakeTable;
use amml_api::testing::{FakeBlock, FakeClient, FakeGame, FakeWorld};
use amml_api::{Aabb, ActorBox, ActorTypeId, BlockHit, BlockPos, BlockTypeId, Face, HitResult, Vec3, WorldInfo};
use levels_core::config::LevelsConfig;
use levels_core::services::InfoPanel;
use levels_core::structs::manager::ManagerSignal;

use crate::support::levels_rig::LevelsRig;
use crate::support::probe::Probe;

// a zombie: kind 0x20 with the mob and monster family bits
const ZOMBIE: ActorTypeId = ActorTypeId(0x0B20);
const PLAYER: ActorTypeId = ActorTypeId(0x013F);
const AT: Vec3 = Vec3 { x: 12.3, y: 64.0, z: -3.2 };
const FEET: BlockPos = BlockPos { x: 12, y: 64, z: -4 };

// an info reader and a shown panel of the default config on a fake game that serves every area of the mod
struct Rig {
    game: FakeGame,
    shared: Arc<Mutex<(InfoReader, InfoPanel)>>,
}

impl Rig {
    fn new() -> Self {
        let game = FakeTable::game();
        for area in LevelsRig::AREAS {
            game.offer_area(area);
        }
        let config = LevelsConfig::default();
        let mut panel = InfoPanel::new(&config.info, config.light, &config.label);
        panel.signal(ManagerSignal::InfoPressed);
        let reader = InfoReader::new(game.ctx());
        Self { game, shared: Arc::new(Mutex::new((reader, panel))) }
    }

    // one tick of the reader at `now` with the client `client`; the lines of the panel after it
    fn tick(&self, client: FakeClient, now: f64) -> Vec<String> {
        let shared = Arc::clone(&self.shared);
        Probe::read(&self.game, client, move |client| {
            let mut guard = shared.lock().unwrap();
            let (reader, panel) = &mut *guard;
            reader.tick(client, panel, now);
            panel.lines().iter().map(|line| line.as_str().to_owned()).collect::<Vec<_>>()
        })
    }

    // the player at `AT` in the overworld; the world of `LevelsRig::world` with `world` laid over it
    fn client_at(world: FakeWorld) -> FakeClient {
        FakeClient { position: Some(AT), world: Some(world), ..LevelsRig::in_world() }
    }

    fn biome_world(id: u16, name: &str) -> FakeWorld {
        FakeWorld {
            biomes_at: HashMap::from([(FEET, id)]),
            biome_names: HashMap::from([(id, name.to_owned())]),
            ..LevelsRig::world()
        }
    }

    fn has(lines: &[String], text: &str) -> bool {
        lines.iter().any(|line| line.contains(text))
    }

    fn actor(x: f32, z: f32) -> ActorBox {
        ActorBox {
            aabb: Aabb { min: Vec3 { x, y: 64.0, z }, max: Vec3 { x: x + 0.6, y: 65.9, z: z + 0.6 } },
            is_local_player: false,
        }
    }
}

#[test]
fn refresh_fills_position_and_biome() {
    let rig = Rig::new();

    let lines = rig.tick(Rig::client_at(Rig::biome_world(7, "deep_dark")), 0.0);

    assert!(Rig::has(&lines, "XYZ: 12.300"), "{lines:?}");
    assert!(Rig::has(&lines, "Biome: deep_dark"), "{lines:?}");
}

#[test]
fn entity_scan_counts_fake_actors() {
    let rig = Rig::new();
    let me = ActorBox { aabb: Aabb { min: AT, max: Vec3 { y: 65.8, ..AT } }, is_local_player: true };
    let world = FakeWorld {
        actors: vec![Rig::actor(14.0, -3.0), Rig::actor(10.0, 2.0), me, Rig::actor(20.0, -10.0)],
        actor_types: vec![ZOMBIE, ZOMBIE, PLAYER, ZOMBIE],
        ..LevelsRig::world()
    };

    let lines = rig.tick(Rig::client_at(world), 0.0);

    assert!(lines.iter().any(|line| line.starts_with("Entities: 3 ")), "{lines:?}");
}

#[test]
fn dimension_change_rereads_world_facts() {
    let rig = Rig::new();
    let named = |name: &str, dimension: i32| FakeClient {
        world_info: Some(WorldInfo { level_name: Some(name.to_owned()), ..WorldInfo::default() }),
        dimension: Some(dimension),
        ..Rig::client_at(LevelsRig::world())
    };
    let first = rig.tick(named("A", 0), 0.0);
    assert!(Rig::has(&first, "World: A"), "{first:?}");

    let lines = rig.tick(named("B", 1), 1.0);

    assert!(Rig::has(&lines, "World: B"), "{lines:?}");
}

#[test]
fn biome_names_clear_on_world_change() {
    let rig = Rig::new();
    let first = rig.tick(Rig::client_at(Rig::biome_world(7, "plains")), 0.0);
    assert!(Rig::has(&first, "Biome: plains"), "{first:?}");

    let nether = FakeClient { dimension: Some(1), ..Rig::client_at(Rig::biome_world(7, "basalt_deltas")) };
    let lines = rig.tick(nether, 1.0);

    assert!(Rig::has(&lines, "Biome: basalt_deltas"), "{lines:?}");
}

#[test]
fn the_target_line_names_the_block_and_its_face() {
    let rig = Rig::new();
    let stone = BlockPos { x: 13, y: 63, z: -4 };
    let world = FakeWorld {
        blocks: HashMap::from([(
            stone,
            FakeBlock { type_id: BlockTypeId(1), name: "minecraft:stone", ..FakeBlock::default() },
        )]),
        ..LevelsRig::world()
    };
    let hit = HitResult::Block(BlockHit { pos: stone, point: Vec3 { x: 13.5, y: 64.0, z: -3.5 }, face: Face::Up });

    let lines = rig.tick(FakeClient { hit_result: Some(hit), ..Rig::client_at(world) }, 0.0);

    assert!(Rig::has(&lines, "Target: minecraft:stone 13 63 -4 face up"), "{lines:?}");
}

#[test]
fn dimension_change_drops_the_old_entity_line_and_refreshes_at_once() {
    let rig = Rig::new();
    let crowded = FakeWorld {
        actors: vec![Rig::actor(14.0, -3.0), Rig::actor(10.0, 2.0)],
        actor_types: vec![ZOMBIE, ZOMBIE],
        ..LevelsRig::world()
    };
    let first = rig.tick(Rig::client_at(crowded), 0.0);
    assert!(Rig::has(&first, "Entities: 2 "), "{first:?}");

    let nether = FakeClient { dimension: Some(1), ..Rig::client_at(LevelsRig::world()) };
    let lines = rig.tick(nether, 0.1);

    assert!(Rig::has(&lines, "Dimension: nether"), "{lines:?}");
    assert!(!Rig::has(&lines, "Entities: 2 "), "{lines:?}");
    assert!(Rig::has(&lines, "Entities: 0 "), "{lines:?}");
}

#[test]
fn world_facts_are_read_again_until_the_game_serves_them() {
    let rig = Rig::new();
    let named = |info: Option<WorldInfo>| FakeClient { world_info: info, ..Rig::client_at(LevelsRig::world()) };
    let first = rig.tick(named(None), 0.0);
    assert!(!Rig::has(&first, "World:"), "{first:?}");

    let info = WorldInfo { level_name: Some("A".to_owned()), ..WorldInfo::default() };
    let lines = rig.tick(named(Some(info)), 5.0);

    assert!(Rig::has(&lines, "World: A"), "{lines:?}");
}
