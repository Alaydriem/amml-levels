use crate::structs::world::ColumnPos;

// y_min and y_max are inclusive
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnRequest {
    pub column: ColumnPos,
    pub y_min: i32,
    pub y_max: i32,
}
