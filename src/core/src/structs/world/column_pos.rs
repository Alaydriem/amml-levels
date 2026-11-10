// a vertical column of blocks, named by its x and z
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ColumnPos {
    pub x: i32,
    pub z: i32,
}
