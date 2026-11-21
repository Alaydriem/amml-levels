use alaydriem_levels::services::FrameStatusReader;
use amm_client::test_support::FakeTable;
use amml_api::testing::{FakeClient, FakeGame};
use amml_api::{Area, Rotation, Vec3};
use levels_core::structs::world::BlockPos;

use crate::support::levels_rig::LevelsRig;
use crate::support::probe::Probe;

fn game() -> FakeGame {
    let game = FakeTable::game();
    for area in [Area::CLIENT_READ, Area::CLIENT_HUD, Area::ACTOR_READ.at(3), Area::WORLD_SKY_DARKEN] {
        game.offer_area(area);
    }
    game
}

fn read(state: FakeClient) -> levels_core::structs::frame::FrameStatus {
    Probe::read(&game(), state, |client| FrameStatusReader::read(client, 3.5))
}

#[test]
fn outside_a_world_only_the_time_is_set() {
    let status = read(FakeClient::default());

    assert_eq!(status.time_seconds, 3.5);
    assert!(!status.in_world && !status.has_player);
}

#[test]
fn a_world_with_no_player_is_in_world_only() {
    let status = read(FakeClient { world_active: true, ..FakeClient::default() });

    assert!(status.in_world);
    assert!(!status.has_player);
}

#[test]
fn the_status_carries_the_pose_the_dimension_and_the_screens() {
    let status = read(FakeClient {
        showing_menu: true,
        hide_hud: true,
        rotation: Some(Rotation { yaw: 90.0, pitch: 10.0 }),
        dimension: Some(1),
        sky_darken: Some(11),
        ..LevelsRig::in_world()
    });

    assert!(status.in_world && status.has_player && status.menu_open && status.hud_hidden);
    assert_eq!((status.dimension, status.sky_darken, status.yaw), (1, 11, 90.0));
    assert_eq!(status.feet, BlockPos { x: 10, y: 64, z: -20 });
}

#[test]
fn the_feet_block_is_the_block_of_the_feet_not_of_the_eyes() {
    let feet = |y: f32| FrameStatusReader::feet_block(Vec3 { x: -0.5, y, z: 0.5 });

    assert_eq!(feet(64.0), BlockPos { x: -1, y: 64, z: 0 });
    assert_eq!(feet(63.9995), BlockPos { x: -1, y: 64, z: 0 });
    assert_eq!(feet(63.5), BlockPos { x: -1, y: 63, z: 0 });
}

#[test]
fn a_player_without_a_readable_position_counts_as_no_player() {
    let status = read(FakeClient { position: None, ..LevelsRig::in_world() });

    assert!(status.in_world);
    assert!(!status.has_player);
}

#[test]
fn the_check_says_whether_the_dimension_of_the_player_was_readable() {
    let checked =
        |state: FakeClient| Probe::read(&game(), state, |client| FrameStatusReader::read_checked(client, 1.0).1);

    assert_eq!(checked(LevelsRig::in_world()), Some(true));
    assert_eq!(checked(FakeClient { dimension: None, ..LevelsRig::in_world() }), Some(false));
    assert_eq!(checked(FakeClient::default()), None);
}
