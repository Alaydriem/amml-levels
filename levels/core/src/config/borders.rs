use serde::Deserialize;

const DEFAULT_EDGE_ALPHA: f32 = 0.69;
const DEFAULT_LINE_WIDTH: f32 = 0.04;
const DEFAULT_EDGE_WIDTH: f32 = 0.08;
const MIN_WIDTH: f32 = 0.01;
const MAX_WIDTH: f32 = 0.5;

// colors are RGBA in 0..=1; widths are in blocks
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct BordersConfig {
    pub line_color: [f32; 4],
    pub edge_alpha: f32,
    pub center_color: [f32; 4],
    pub line_width: f32,
    pub edge_width: f32,
}

impl Default for BordersConfig {
    fn default() -> Self {
        Self {
            line_color: [1.0, 1.0, 1.0, 0.6],
            edge_alpha: DEFAULT_EDGE_ALPHA,
            center_color: [1.0, 1.0, 1.0, 0.6],
            line_width: DEFAULT_LINE_WIDTH,
            edge_width: DEFAULT_EDGE_WIDTH,
        }
    }
}

impl BordersConfig {
    pub fn sanitized(self) -> Self {
        Self {
            line_color: Self::color(self.line_color),
            edge_alpha: Self::unit(self.edge_alpha, DEFAULT_EDGE_ALPHA),
            center_color: Self::color(self.center_color),
            line_width: Self::width(self.line_width, DEFAULT_LINE_WIDTH),
            edge_width: Self::width(self.edge_width, DEFAULT_EDGE_WIDTH),
        }
    }

    fn color(value: [f32; 4]) -> [f32; 4] {
        value.map(|c| Self::unit(c, 1.0))
    }

    fn unit(value: f32, fallback: f32) -> f32 {
        if value.is_finite() { value.clamp(0.0, 1.0) } else { fallback }
    }

    fn width(value: f32, fallback: f32) -> f32 {
        if value.is_finite() { value.clamp(MIN_WIDTH, MAX_WIDTH) } else { fallback }
    }
}
