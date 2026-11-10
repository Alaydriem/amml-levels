use serde::Deserialize;

const DEFAULT_SCALE: f32 = 1.0;
const DEFAULT_LIFT: f32 = 0.02;
// below the top of a carpet (1/16 block), so carpets and snow layers over a block hide its label
const MAX_LIFT: f32 = 0.05;

// colors are RGBA in 0..=1; lift raises the text above the top face so it does not z-fight
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct LabelConfig {
    pub red: [f32; 4],
    pub yellow: [f32; 4],
    pub green: [f32; 4],
    pub lift: f32,
    pub scale: f32,
}

impl Default for LabelConfig {
    fn default() -> Self {
        Self {
            red: [1.0, 0.25, 0.25, 1.0],
            yellow: [1.0, 0.85, 0.2, 1.0],
            green: [0.35, 1.0, 0.35, 1.0],
            lift: DEFAULT_LIFT,
            scale: DEFAULT_SCALE,
        }
    }
}

impl LabelConfig {
    pub fn sanitized(self) -> Self {
        Self {
            red: Self::color(self.red),
            yellow: Self::color(self.yellow),
            green: Self::color(self.green),
            lift: if (0.0..=MAX_LIFT).contains(&self.lift) { self.lift } else { DEFAULT_LIFT },
            scale: if self.scale.is_finite() && self.scale > 0.0 { self.scale } else { DEFAULT_SCALE },
        }
    }

    fn color(value: [f32; 4]) -> [f32; 4] {
        value.map(|c| if c.is_finite() { c.clamp(0.0, 1.0) } else { 1.0 })
    }
}
