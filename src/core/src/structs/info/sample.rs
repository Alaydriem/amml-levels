use crate::structs::info::InfoTarget;
use crate::structs::world::Vec3f;

// the player facts one panel refresh reads; yaw and pitch are in degrees
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InfoSample {
    pub time_seconds: f64,
    pub position: Vec3f,
    pub yaw: f32,
    pub pitch: f32,
    pub dimension: i32,
    pub biome: Option<u16>,
    pub sky_darken: u8,
    pub target: InfoTarget,
}
