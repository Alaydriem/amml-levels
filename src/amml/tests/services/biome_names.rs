use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use alaydriem_levels::services::BiomeNames;
use amm_client::test_support::FakeTable;
use amml_api::testing::{FakeClient, FakeWorld};

use crate::support::levels_rig::LevelsRig;
use crate::support::probe::Probe;

struct Fixture;

impl Fixture {
    // the player of `LevelsRig::in_world` in a world that names every biome id up to `CAPACITY` as `<prefix><id>`
    fn naming(prefix: &str) -> FakeClient {
        let names: HashMap<u16, String> =
            (0..=BiomeNames::CAPACITY as u16).map(|id| (id, format!("{prefix}{id}"))).collect();
        FakeClient { world: Some(FakeWorld { biome_names: names, ..LevelsRig::world() }), ..LevelsRig::in_world() }
    }
}

#[test]
fn biome_names_evicts_lru() {
    let game = FakeTable::game();
    for area in LevelsRig::AREAS {
        game.offer_area(area);
    }
    let biomes = game.ctx().biomes();
    let names = Arc::new(Mutex::new(BiomeNames::new()));
    let last = BiomeNames::CAPACITY as u16;

    let filling = Arc::clone(&names);
    Probe::read(&game, Fixture::naming("old"), move |_| {
        let mut names = filling.lock().unwrap();
        for id in 0..last {
            assert_eq!(names.name(&biomes, id), Some(format!("old{id}").as_str()));
        }
        names.name(&biomes, 0);
        names.name(&biomes, last);
    });
    let reading = Arc::clone(&names);
    let (touched, newest, untouched) = Probe::read(&game, Fixture::naming("new"), move |_| {
        let mut names = reading.lock().unwrap();
        let touched = names.name(&biomes, 0).map(str::to_owned);
        let newest = names.name(&biomes, last).map(str::to_owned);
        let untouched = names.name(&biomes, 1).map(str::to_owned);
        (touched, newest, untouched)
    });

    assert_eq!(touched.as_deref(), Some("old0"), "the first id was used again before the cache filled, so it stays");
    assert_eq!(newest.as_deref(), Some(format!("old{last}").as_str()), "the newest id stays");
    assert_eq!(untouched.as_deref(), Some("new1"), "the least recently used id was evicted and read again");
}
