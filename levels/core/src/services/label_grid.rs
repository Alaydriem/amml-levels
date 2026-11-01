use crate::config::MAX_COLUMN_SURFACES;
use crate::structs::light::SurfaceSample;
use crate::structs::world::{BlockPos, ColumnPos};

// the latest surfaces of every column around the player, in a fixed (2r + 1)^2 slot grid.
// A column maps to slot (x mod side, z mod side), so a new column evicts the one that held its slot,
// and each slot holds at most MAX_COLUMN_SURFACES samples. Memory is allocated once in new().
pub struct LabelGrid {
    side: i32,
    columns: Vec<Option<ColumnPos>>,
    lens: Vec<usize>,
    samples: Vec<SurfaceSample>,
    total: usize,
}

impl LabelGrid {
    pub fn new(radius: u32) -> Self {
        let side = 2 * radius as i32 + 1;
        let slots = (side * side) as usize;
        Self {
            side,
            columns: vec![None; slots],
            lens: vec![0; slots],
            samples: vec![SurfaceSample::default(); slots * MAX_COLUMN_SURFACES],
            total: 0,
        }
    }

    // replaces what the column's slot held; samples past MAX_COLUMN_SURFACES are dropped
    pub fn store(&mut self, column: ColumnPos, samples: &[SurfaceSample]) {
        let slot = self.slot(column);
        let kept = samples.len().min(MAX_COLUMN_SURFACES);
        let start = slot * MAX_COLUMN_SURFACES;
        self.samples[start..start + kept].copy_from_slice(&samples[..kept]);
        self.total = self.total - self.lens[slot] + kept;
        self.lens[slot] = kept;
        self.columns[slot] = Some(column);
    }

    pub fn clear(&mut self) {
        self.columns.fill(None);
        self.lens.fill(0);
        self.total = 0;
    }

    pub fn resident_bytes(&self) -> u64 {
        let slots = self.columns.capacity() * std::mem::size_of::<Option<ColumnPos>>()
            + self.lens.capacity() * std::mem::size_of::<usize>();
        (slots + self.samples.capacity() * std::mem::size_of::<SurfaceSample>()) as u64
    }

    pub fn len(&self) -> usize {
        self.total
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    // samples of the stored columns whose x/z lie within the disk of radius around feet
    pub fn samples_near(&self, feet: BlockPos, radius: u32) -> impl Iterator<Item = &SurfaceSample> {
        let limit = (radius as i64) * (radius as i64);
        self.columns.iter().enumerate().flat_map(move |(slot, column)| {
            let near = column.is_some_and(|c| {
                let dx = (c.x - feet.x) as i64;
                let dz = (c.z - feet.z) as i64;
                dx * dx + dz * dz <= limit
            });
            let len = if near { self.lens[slot] } else { 0 };
            let start = slot * MAX_COLUMN_SURFACES;
            self.samples[start..start + len].iter()
        })
    }

    fn slot(&self, column: ColumnPos) -> usize {
        (column.x.rem_euclid(self.side) * self.side + column.z.rem_euclid(self.side)) as usize
    }
}
