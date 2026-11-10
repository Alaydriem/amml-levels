use crate::structs::world::ColumnPos;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub fn column(&self) -> ColumnPos {
        ColumnPos { x: self.x, z: self.z }
    }
}
