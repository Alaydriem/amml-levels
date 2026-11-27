use std::collections::HashMap;

use alaydriem_levels::services::SurfaceSampler;
use amm_client::test_support::FakeTable;
use amml_api::testing::{FakeChunk, FakeClient, FakeGame, FakeWorld};
use amml_api::{Area, BlockPos, ChunkLoad, SurfaceSample};
use levels_core::config::MAX_COLUMN_SURFACES;
use levels_core::structs::frame::ColumnRequest;
use levels_core::structs::light::SurfaceSample as CoreSample;
use levels_core::structs::world::{BlockPos as CoreBlockPos, ColumnPos};

use crate::support::levels_rig::LevelsRig;
use crate::support::probe::Probe;

struct Fixture;

impl Fixture {
    fn game() -> FakeGame {
        let game = FakeTable::game();
        for area in [Area::CLIENT_READ, Area::ACTOR_READ.at(3), Area::WORLD_SCAN_SURFACES] {
            game.offer_area(area);
        }
        game
    }

    fn surface(x: i32, y: i32, z: i32, block_light: u8, sky_light: u8) -> SurfaceSample {
        SurfaceSample { pos: BlockPos { x, y, z }, block_light, sky_light }
    }

    fn world(chunk: ChunkLoad, surfaces: Vec<SurfaceSample>) -> FakeWorld {
        FakeWorld {
            chunks: HashMap::from([((0, 0), FakeChunk::new(chunk))]),
            surfaces: HashMap::from([((3, 4), surfaces)]),
            ..FakeWorld::default()
        }
    }

    // the outcome and the samples of one request of the column (3, 4) between y -64 and y 320
    fn sample(world: FakeWorld) -> (alaydriem_levels::types::SampleOutcome, Vec<CoreSample>) {
        let client = FakeClient { world: Some(world), ..LevelsRig::in_world() };
        Probe::read(&Fixture::game(), client, |client| {
            let region = client.local_player().unwrap().block_source().unwrap();
            let request = ColumnRequest { column: ColumnPos { x: 3, z: 4 }, y_min: -64, y_max: 320 };
            let mut out = vec![CoreSample::default()];
            let outcome = SurfaceSampler::new().sample(&region, &request, &mut out);
            (outcome, out)
        })
    }
}

#[test]
fn the_surfaces_of_a_loaded_column_come_back_lowest_first_with_their_light() {
    let (outcome, out) = Fixture::sample(Fixture::world(
        ChunkLoad::Loaded,
        vec![Fixture::surface(3, 10, 4, 7, 0), Fixture::surface(3, 40, 4, 0, 15)],
    ));

    assert_eq!(outcome, alaydriem_levels::types::SampleOutcome::Sampled(2));
    assert_eq!(
        out,
        [
            CoreSample { pos: CoreBlockPos { x: 3, y: 10, z: 4 }, block_light: 7, sky_light: 0 },
            CoreSample { pos: CoreBlockPos { x: 3, y: 40, z: 4 }, block_light: 0, sky_light: 15 },
        ]
    );
}

#[test]
fn a_column_in_an_unloaded_chunk_is_skipped_and_gives_no_samples() {
    let (outcome, out) = Fixture::sample(Fixture::world(ChunkLoad::NotLoaded, vec![Fixture::surface(3, 10, 4, 7, 0)]));

    assert_eq!(outcome, alaydriem_levels::types::SampleOutcome::Skipped);
    assert!(out.is_empty());
}

#[test]
fn a_column_with_no_surface_is_sampled_empty() {
    let (outcome, out) = Fixture::sample(Fixture::world(ChunkLoad::Loaded, Vec::new()));

    assert_eq!(outcome, alaydriem_levels::types::SampleOutcome::Sampled(0));
    assert!(out.is_empty());
}

#[test]
fn no_more_than_the_column_surface_limit_is_read() {
    let many = (0..(MAX_COLUMN_SURFACES as i32 + 10)).map(|y| Fixture::surface(3, y, 4, 1, 1)).collect();

    let (outcome, out) = Fixture::sample(Fixture::world(ChunkLoad::Loaded, many));

    assert_eq!(outcome, alaydriem_levels::types::SampleOutcome::Sampled(MAX_COLUMN_SURFACES));
    assert_eq!(out.len(), MAX_COLUMN_SURFACES);
}

#[test]
fn the_request_heights_bound_the_surfaces() {
    let client = FakeClient {
        world: Some(Fixture::world(
            ChunkLoad::Loaded,
            vec![Fixture::surface(3, 10, 4, 7, 0), Fixture::surface(3, 40, 4, 0, 15)],
        )),
        ..LevelsRig::in_world()
    };

    let out = Probe::read(&Fixture::game(), client, |client| {
        let region = client.local_player().unwrap().block_source().unwrap();
        let request = ColumnRequest { column: ColumnPos { x: 3, z: 4 }, y_min: 20, y_max: 60 };
        let mut out = Vec::new();
        SurfaceSampler::new().sample(&region, &request, &mut out);
        out
    });

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].pos.y, 40);
}
