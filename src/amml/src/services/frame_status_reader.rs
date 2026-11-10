use std::sync::OnceLock;
use std::time::Instant;

use amml_game::ClientInstance;
use levels_core::structs::frame::FrameStatus;
use levels_core::structs::world::BlockPos;

// reads the game state core needs once per frame
pub struct FrameStatusReader;

impl FrameStatusReader {
    // the height of the player's eyes above the feet: the pose position is the eye position
    const EYE_HEIGHT: f32 = 1.62;

    // keeps a player standing exactly on a block edge in the block above it
    const FEET_BIAS: f32 = 1e-3;

    // seconds since the first call; a steady clock
    pub fn now_seconds() -> f64 {
        static START: OnceLock<Instant> = OnceLock::new();
        START.get_or_init(Instant::now).elapsed().as_secs_f64()
    }

    // the block the player's feet are in, from the pose position
    pub fn feet_block(pos: [f32; 3]) -> BlockPos {
        BlockPos {
            x: pos[0].floor() as i32,
            y: (pos[1] - Self::EYE_HEIGHT + Self::FEET_BIAS).floor() as i32,
            z: pos[2].floor() as i32,
        }
    }

    // the status of this frame. Outside a world only `time_seconds` is set; in a world with no player (as during a
    // dimension change) `in_world` is set and the rest is not
    pub fn read(client: &ClientInstance<'_>, now: f64) -> FrameStatus {
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
        if !client.is_world_active() {
            return status;
        }
        status.in_world = true;
        let Some(player) = client.local_player() else { return status };
        let Some(pose) = player.pose() else { return status };
        status.has_player = true;
        status.menu_open = client.is_showing_menu();
        status.hud_hidden = client.hide_hud().unwrap_or(false);
        let dimension = player.dimension();
        status.dimension = dimension.as_ref().and_then(|dimension| dimension.id()).unwrap_or(0);
        status.sky_darken = dimension.as_ref().and_then(|dimension| dimension.sky_darken()).unwrap_or(0);
        status.feet = Self::feet_block(pose.pos);
        status.yaw = pose.yaw;
        status
    }
}
