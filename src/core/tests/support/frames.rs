use levels_core::structs::frame::FrameStatus;
use levels_core::structs::world::BlockPos;

pub const FRAME: f64 = 1.0 / 60.0;

pub fn feet() -> BlockPos {
    BlockPos { x: 10, y: 64, z: -20 }
}

// an in-world, no-menu status in the overworld at frame `index`
pub fn in_world(index: u32) -> FrameStatus {
    FrameStatus {
        time_seconds: index as f64 * FRAME,
        in_world: true,
        has_player: true,
        menu_open: false,
        hud_hidden: false,
        dimension: 0,
        feet: feet(),
        sky_darken: 0,
        yaw: 0.0,
    }
}

pub fn out_of_world(index: u32) -> FrameStatus {
    FrameStatus { in_world: false, has_player: false, ..in_world(index) }
}
