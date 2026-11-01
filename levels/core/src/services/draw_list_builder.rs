use crate::config::{GAME_TAG_HEADROOM, LabelConfig, MAX_COLUMN_SURFACES, ScanConfig};
use crate::services::{LabelGrid, LightClassifier};
use crate::structs::draw::{CameraView, Label};
use crate::structs::light::{LightBand, SurfaceSample};
use crate::structs::world::BlockPos;

// turns the stored samples into the frame's labels: within the sphere, the nearest kept. The cap is
// max_labels, lowered by every game name tag past GAME_TAG_HEADROOM, so the drawn list stays bounded.
// Labels behind the camera are left out before the cap, so the cap is spent on labels that can be seen.
// Both buffers are allocated once in new() and reused every frame.
pub struct DrawListBuilder {
    radius: u32,
    max_labels: usize,
    label: LabelConfig,
    classifier: LightClassifier,
    // (distance squared to the feet, sample)
    candidates: Vec<(i64, SurfaceSample)>,
    labels: Vec<Label>,
    // labels left out behind the camera by the last build
    culled: usize,
}

impl DrawListBuilder {
    pub fn new(scan: &ScanConfig, label: LabelConfig, classifier: LightClassifier) -> Self {
        let side = 2 * scan.radius as usize + 1;
        let max_labels = scan.max_labels.max(1) as usize;
        Self {
            radius: scan.radius,
            max_labels,
            label,
            classifier,
            candidates: Vec::with_capacity(side * side * MAX_COLUMN_SURFACES),
            labels: Vec::with_capacity(max_labels),
            culled: 0,
        }
    }

    // replaces the label look and the light bands; the buffers stay as sized by new
    pub fn restyle(&mut self, label: LabelConfig, classifier: LightClassifier) {
        self.label = label;
        self.classifier = classifier;
    }

    pub fn max_labels(&self) -> usize {
        self.max_labels
    }

    // the labels of the last build
    pub fn labels(&self) -> &[Label] {
        &self.labels
    }

    pub fn culled(&self) -> usize {
        self.culled
    }

    pub fn scale(&self) -> f32 {
        self.label.scale
    }

    pub fn resident_bytes(&self) -> u64 {
        (self.candidates.capacity() * std::mem::size_of::<(i64, SurfaceSample)>()
            + self.labels.capacity() * std::mem::size_of::<Label>()) as u64
    }

    // game_tags is the number of name tags the game itself draws this frame; with no camera every side is drawn
    pub fn build(
        &mut self,
        grid: &LabelGrid,
        feet: BlockPos,
        sky_darken: u8,
        game_tags: usize,
        camera: Option<CameraView>,
    ) -> &[Label] {
        let cap = self.max_labels.saturating_sub(game_tags.saturating_sub(GAME_TAG_HEADROOM));
        let limit = (self.radius as i64) * (self.radius as i64);
        self.candidates.clear();
        self.culled = 0;
        for sample in grid.samples_near(feet, self.radius) {
            let distance = Self::distance_squared(sample.pos, feet);
            if distance > limit {
                continue;
            }
            if camera.is_some_and(|camera| camera.is_behind(self.position(sample))) {
                self.culled += 1;
                continue;
            }
            self.candidates.push((distance, *sample));
        }
        if self.candidates.len() > cap {
            self.candidates.select_nth_unstable_by_key(cap, |c| c.0);
            self.candidates.truncate(cap);
        }

        self.labels.clear();
        for (_, sample) in &self.candidates {
            let value = LightClassifier::light(sample.block_light, sample.sky_light, sky_darken);
            let [x, y, z] = self.position(sample);
            self.labels.push(Label { x, y, z, value, color: self.color(self.classifier.classify(value)) });
        }
        &self.labels
    }

    // the top-face centre, lifted
    fn position(&self, sample: &SurfaceSample) -> [f32; 3] {
        [sample.pos.x as f32 + 0.5, sample.pos.y as f32 + 1.0 + self.label.lift, sample.pos.z as f32 + 0.5]
    }

    fn color(&self, band: LightBand) -> [f32; 4] {
        match band {
            LightBand::Red => self.label.red,
            LightBand::Yellow => self.label.yellow,
            LightBand::Green => self.label.green,
        }
    }

    fn distance_squared(a: BlockPos, b: BlockPos) -> i64 {
        let dx = (a.x - b.x) as i64;
        let dy = (a.y - b.y) as i64;
        let dz = (a.z - b.z) as i64;
        dx * dx + dy * dy + dz * dz
    }
}
