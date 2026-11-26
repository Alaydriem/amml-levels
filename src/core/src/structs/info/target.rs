use crate::structs::world::{BlockPos, Vec3f};

// what the crosshair points at; `face` is the game's block face index
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum InfoTarget {
    #[default]
    None,
    Block {
        pos: BlockPos,
        face: u8,
    },
    Entity {
        point: Vec3f,
    },
}
