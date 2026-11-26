use crate::structs::world::Vec3f;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ActorSample {
    pub min: Vec3f,
    pub max: Vec3f,
    pub actor_type: u32,
    pub is_local_player: bool,
}
