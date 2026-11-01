use crate::structs::world::BlockPos;

// what the host reads from the game once per frame
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStatus {
    pub time_seconds: f64,
    pub in_world: bool,
    // false for a tick in a world with no local player, as during a dimension change; the other fields are unset
    pub has_player: bool,
    // any screen is open: chat, a sign, a book, the pause menu
    pub menu_open: bool,
    // the game's HUD is hidden (F1, or a mod such as Freecam); the game then draws no name tags
    pub hud_hidden: bool,
    pub dimension: i32,
    pub feet: BlockPos,
    pub sky_darken: u8,
    // degrees, as LocalPlayer::getRotation().y
    pub yaw: f32,
}
