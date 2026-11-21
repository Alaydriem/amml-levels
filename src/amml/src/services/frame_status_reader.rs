use std::sync::OnceLock;
use std::time::Instant;

use amml_api::{Client, Vec3};
use levels_core::structs::frame::FrameStatus;
use levels_core::structs::world::BlockPos;

// reads the game state core needs once per frame
pub struct FrameStatusReader;

impl FrameStatusReader {
    // keeps a player standing exactly on a block edge in the block above it
    const FEET_BIAS: f32 = 1e-3;

    // seconds since the first call; a steady clock
    pub fn now_seconds() -> f64 {
        static START: OnceLock<Instant> = OnceLock::new();
        START.get_or_init(Instant::now).elapsed().as_secs_f64()
    }

    // the block the player's feet are in, from the feet position
    pub fn feet_block(feet: Vec3) -> BlockPos {
        BlockPos { x: feet.x.floor() as i32, y: (feet.y + Self::FEET_BIAS).floor() as i32, z: feet.z.floor() as i32 }
    }

    // the status of this frame. Outside a world only `time_seconds` is set; in a world with no player (as during a
    // dimension change) `in_world` is set and the rest is not. The yaw is the loader's `Rotation` yaw: 0 faces +Z and
    // it grows clockwise, the game's own heading
    pub fn read(client: &Client<'_>, now: f64) -> FrameStatus {
        Self::read_checked(client, now).0
    }

    // `read`, and whether the dimension of the player was readable: `None` while there is no player to read it from
    pub fn read_checked(client: &Client<'_>, now: f64) -> (FrameStatus, Option<bool>) {
        let mut status = FrameStatus {
            time_seconds: now,
            in_world: false,
            has_player: false,
            menu_open: false,
            hud_hidden: false,
            dimension: 0,
            feet: BlockPos::default(),
            sky_darken: 0,
            yaw: 0.0,
        };
        if client.world_active() != Some(true) {
            return (status, None);
        }
        status.in_world = true;
        let Some(player) = client.local_player() else { return (status, None) };
        let (Some(feet), Some(rotation)) = (player.position(), player.rotation()) else { return (status, None) };
        status.has_player = true;
        status.menu_open = client.showing_menu().unwrap_or(false);
        status.hud_hidden = client.hide_hud().unwrap_or(false);
        let dimension = player.dimension();
        let id = dimension.as_ref().and_then(|dimension| dimension.id());
        status.dimension = id.unwrap_or(0);
        status.sky_darken = dimension.as_ref().and_then(|dimension| dimension.sky_darken()).unwrap_or(0);
        status.feet = Self::feet_block(feet);
        status.yaw = rotation.yaw;
        (status, Some(id.is_some()))
    }
}
