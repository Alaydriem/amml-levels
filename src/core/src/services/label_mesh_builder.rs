use crate::config::{MAX_LABELS, MAX_RADIUS};
use crate::services::DigitFont;
use crate::structs::draw::{ColorQuad, Label, LabelPart, PartHash};
use crate::structs::world::BlockPos;

const QUARTER_DEGREES: f32 = 90.0;
const PIXEL_BLOCKS: f32 = 0.04;
const DIGIT_GAP_PIXELS: usize = 1;

// Flat digit quads for the light labels. Capacity is fixed at construction: `max_labels` labels and
// `max_labels * MAX_QUADS_PER_LABEL` quads; labels past `max_labels` are dropped. The quads are cut into parts of whole
// labels: the labels are grouped by the world-aligned cell of `CELL_BLOCKS` x `CELL_BLOCKS` columns they stand in, and a
// part holds the labels of one cell, or 511 of them when the cell has more. A change in one place leaves the parts of the
// other cells (and their keys) as they were, and a step of the player changes only the cells at the edge of the disk.
// Past `MAX_PARTS` parts the remaining labels are dropped.
pub struct LabelMeshBuilder {
    max_labels: usize,
    quads: Vec<ColorQuad>,
    parts: Vec<LabelPart>,
    max_parts: usize,
    // (cell, index into the labels) of the labels of the build in progress, sorted by cell
    order: Vec<(u64, u32)>,
    last_labels: Vec<Label>,
    last_quarter: u8,
    last_scale: f32,
    last_origin: BlockPos,
    built: bool,
    origin: BlockPos,
    version: u64,
}

impl LabelMeshBuilder {
    pub const MAX_QUADS_PER_LABEL: usize = 16;
    // side in blocks of the square of columns one part covers
    pub const CELL_BLOCKS: i32 = 16;
    // keeps a part under the quads one mesh of the loader holds (8191)
    pub const MAX_LABELS_PER_PART: usize = 511;
    // the most parts any build gives: a break at a cell edge or at the label limit
    pub const MAX_PARTS: usize = Self::parts_for(MAX_LABELS as usize);

    const fn parts_for(labels: usize) -> usize {
        let cells = (2 * MAX_RADIUS as usize + 1).div_ceil(Self::CELL_BLOCKS as usize) + 1;
        labels.div_ceil(Self::MAX_LABELS_PER_PART) + cells * cells
    }

    // round half away from zero, then wrap to 0..=3
    pub fn quarter_for_yaw(yaw_degrees: f32) -> u8 {
        ((yaw_degrees / QUARTER_DEGREES).round() as i64).rem_euclid(4) as u8
    }

    pub fn new(max_labels: usize) -> Self {
        Self {
            max_labels,
            quads: Vec::with_capacity(max_labels * Self::MAX_QUADS_PER_LABEL),
            parts: Vec::with_capacity(Self::parts_for(max_labels)),
            max_parts: Self::parts_for(max_labels),
            order: Vec::with_capacity(max_labels),
            last_labels: Vec::with_capacity(max_labels),
            last_quarter: 0,
            last_scale: 0.0,
            last_origin: BlockPos::default(),
            built: false,
            origin: BlockPos::default(),
            version: 0,
        }
    }

    // `origin` is the block the quads are relative to. While the caller keeps it, the keys of unchanged parts stay equal
    pub fn build(&mut self, labels: &[Label], origin: BlockPos, yaw_degrees: f32, scale: f32) -> u64 {
        let labels = &labels[..labels.len().min(self.max_labels)];
        let quarter = Self::quarter_for_yaw(yaw_degrees);
        if self.built
            && quarter == self.last_quarter
            && scale == self.last_scale
            && origin == self.last_origin
            && labels == self.last_labels.as_slice()
        {
            return self.version;
        }
        self.built = true;
        self.last_quarter = quarter;
        self.last_scale = scale;
        self.last_origin = origin;
        self.last_labels.clear();
        self.last_labels.extend_from_slice(labels);
        self.quads.clear();
        self.parts.clear();
        self.origin = origin;
        let mut hash = PartHash::new([origin.x, origin.y, origin.z], quarter, scale);
        self.order.clear();
        for (index, label) in labels.iter().enumerate() {
            self.order.push((Self::cell_of(label), index as u32));
        }
        self.order.sort_unstable();
        let mut start = 0;
        let mut count = 0;
        let mut cell = 0;
        for position in 0..self.order.len() {
            let (label_cell, index) = self.order[position];
            if count > 0 && (label_cell != cell || count == Self::MAX_LABELS_PER_PART) {
                if self.parts.len() + 1 >= self.max_parts {
                    break;
                }
                self.parts.push(LabelPart { key: hash.finish(), start, end: self.quads.len() });
                start = self.quads.len();
                count = 0;
                hash.restart();
            }
            cell = label_cell;
            let label = &labels[index as usize];
            self.push_label(label, quarter, scale);
            hash.push(label);
            count += 1;
        }
        if count > 0 {
            self.parts.push(LabelPart { key: hash.finish(), start, end: self.quads.len() });
        }
        self.version += 1;
        self.version
    }

    // the world-aligned cell of the label, x in the high word
    fn cell_of(label: &Label) -> u64 {
        let word = |value: f32| ((value.floor() as i32).div_euclid(Self::CELL_BLOCKS) as i64 + (1 << 31)) as u64;
        word(label.x) << 32 | word(label.z)
    }

    // the cut of the quads of the last build into meshes
    pub fn parts(&self) -> &[LabelPart] {
        &self.parts
    }

    pub fn quads(&self) -> &[ColorQuad] {
        &self.quads
    }

    pub fn origin(&self) -> BlockPos {
        self.origin
    }

    pub fn resident_bytes(&self) -> u64 {
        (self.quads.capacity() * std::mem::size_of::<ColorQuad>()
            + self.parts.capacity() * std::mem::size_of::<LabelPart>()
            + self.order.capacity() * std::mem::size_of::<(u64, u32)>()
            + self.last_labels.capacity() * std::mem::size_of::<Label>()) as u64
    }

    fn push_label(&mut self, label: &Label, quarter: u8, scale: f32) {
        // (x, z) components of the up-the-page vector U and the right vector R
        let (up, right) = match quarter {
            0 => ([0.0_f32, 1.0], [-1.0_f32, 0.0]),
            1 => ([-1.0, 0.0], [0.0, -1.0]),
            2 => ([0.0, -1.0], [1.0, 0.0]),
            _ => ([1.0, 0.0], [0.0, 1.0]),
        };
        let rgba = label.color.map(|c| (c * 255.0).round().clamp(0.0, 255.0) as u8);
        let pixel = PIXEL_BLOCKS * scale;
        let (tens, ones) = (label.value / 10 % 10, label.value % 10);
        let count = if label.value >= 10 { 2 } else { 1 };
        let width = count * DigitFont::WIDTH as usize + (count - 1) * DIGIT_GAP_PIXELS;
        let cx = label.x - self.origin.x as f32;
        let cy = label.y - self.origin.y as f32;
        let cz = label.z - self.origin.z as f32;
        let digits = [tens, ones];
        let digits = &digits[2 - count..];
        for (index, digit) in digits.iter().enumerate() {
            let digit_start = index * (DigitFont::WIDTH as usize + DIGIT_GAP_PIXELS);
            for &(row, column, length) in DigitFont::runs(*digit) {
                let left = (digit_start + column as usize) as f32 - width as f32 / 2.0;
                let right_edge = left + length as f32;
                let top = DigitFont::HEIGHT as f32 / 2.0 - row as f32;
                let bottom = top - 1.0;
                let corner = |h: f32, v: f32| {
                    [cx + right[0] * h * pixel + up[0] * v * pixel, cy, cz + right[1] * h * pixel + up[1] * v * pixel]
                };
                self.quads.push(ColorQuad {
                    corners: [
                        corner(left, top),
                        corner(right_edge, top),
                        corner(right_edge, bottom),
                        corner(left, bottom),
                    ],
                    rgba,
                });
            }
        }
    }
}
