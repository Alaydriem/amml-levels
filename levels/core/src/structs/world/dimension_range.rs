const OVERWORLD: DimensionRange = DimensionRange { min_y: -64, max_y: 320 };
const NETHER: DimensionRange = DimensionRange { min_y: 0, max_y: 128 };
const END: DimensionRange = DimensionRange { min_y: 0, max_y: 256 };

// build height limits of a dimension, both ends inclusive
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DimensionRange {
    pub min_y: i32,
    pub max_y: i32,
}

impl DimensionRange {
    pub fn for_dimension(id: i32) -> Self {
        match id {
            1 => NETHER,
            2 => END,
            _ => OVERWORLD,
        }
    }
}
