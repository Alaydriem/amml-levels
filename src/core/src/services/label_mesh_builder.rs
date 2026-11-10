use crate::services::DigitFont;
use crate::structs::draw::{ColorQuad, Label};
use crate::structs::world::BlockPos;

const QUARTER_DEGREES: f32 = 90.0;
const PIXEL_BLOCKS: f32 = 0.04;
const DIGIT_GAP_PIXELS: usize = 1;

// Flat digit quads for the light labels. Capacity is fixed at construction: `max_labels` labels and
// `max_labels * MAX_QUADS_PER_LABEL` quads; labels past `max_labels` are dropped.
pub struct LabelMeshBuilder {
    max_labels: usize,
    quads: Vec<ColorQuad>,
    last_labels: Vec<Label>,
    last_quarter: u8,
    last_scale: f32,
    built: bool,
    origin: BlockPos,
    version: u64,
}

impl LabelMeshBuilder {
    pub const MAX_QUADS_PER_LABEL: usize = 16;

    // round half away from zero, then wrap to 0..=3
    pub fn quarter_for_yaw(yaw_degrees: f32) -> u8 {
        ((yaw_degrees / QUARTER_DEGREES).round() as i64).rem_euclid(4) as u8
    }

    pub fn new(max_labels: usize) -> Self {
        Self {
            max_labels,
            quads: Vec::with_capacity(max_labels * Self::MAX_QUADS_PER_LABEL),
            last_labels: Vec::with_capacity(max_labels),
            last_quarter: 0,
            last_scale: 0.0,
            built: false,
            origin: BlockPos::default(),
            version: 0,
        }
    }

    pub fn build(&mut self, labels: &[Label], yaw_degrees: f32, scale: f32) -> u64 {
        let labels = &labels[..labels.len().min(self.max_labels)];
        let quarter = Self::quarter_for_yaw(yaw_degrees);
        if self.built
            && quarter == self.last_quarter
            && scale == self.last_scale
            && labels == self.last_labels.as_slice()
        {
            return self.version;
        }
        self.built = true;
        self.last_quarter = quarter;
        self.last_scale = scale;
        self.last_labels.clear();
        self.last_labels.extend_from_slice(labels);
        self.quads.clear();
        self.origin = match labels.first() {
            Some(first) => BlockPos { x: first.x.floor() as i32, y: first.y.floor() as i32, z: first.z.floor() as i32 },
            None => BlockPos::default(),
        };
        for label in labels {
            self.push_label(label, quarter, scale);
        }
        self.version += 1;
        self.version
    }

    pub fn quads(&self) -> &[ColorQuad] {
        &self.quads
    }

    pub fn origin(&self) -> BlockPos {
        self.origin
    }

    pub fn resident_bytes(&self) -> u64 {
        (self.quads.capacity() * std::mem::size_of::<ColorQuad>()
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
