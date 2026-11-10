use serde::Deserialize;

const DEFAULT_MAX_MARKERS: u32 = 16;
pub const MAX_MARKERS: u32 = 64;
const DEFAULT_SHOW_DISTANCE: f32 = 96.0;
const MIN_SHOW_DISTANCE: f32 = 8.0;
const MAX_SHOW_DISTANCE: f32 = 256.0;
const DEFAULT_INNER_RADIUS: f32 = 24.0;
const DEFAULT_OUTER_RADIUS: f32 = 44.5;
const MIN_RADIUS: f32 = 1.0;
const MAX_RADIUS: f32 = 128.0;
const DEFAULT_SPIN: f32 = 2.5;
const MAX_SPIN: f32 = 90.0;

// colors are RGBA in 0..=1; radii are in blocks; the spin is in degrees per second
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct SpheresConfig {
    pub max_markers: u32,
    pub show_distance: f32,
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub inner_color: [f32; 4],
    pub outer_color: [f32; 4],
    pub spin: f32,
}

impl Default for SpheresConfig {
    fn default() -> Self {
        Self {
            max_markers: DEFAULT_MAX_MARKERS,
            show_distance: DEFAULT_SHOW_DISTANCE,
            inner_radius: DEFAULT_INNER_RADIUS,
            outer_radius: DEFAULT_OUTER_RADIUS,
            inner_color: [0.0, 1.0, 0.008, 0.08],
            outer_color: [1.0, 0.02, 0.008, 0.08],
            spin: DEFAULT_SPIN,
        }
    }
}

impl SpheresConfig {
    pub fn sanitized(self) -> Self {
        let inner_radius = Self::clamped(self.inner_radius, MIN_RADIUS, MAX_RADIUS, DEFAULT_INNER_RADIUS);
        let outer_radius = Self::clamped(self.outer_radius, MIN_RADIUS, MAX_RADIUS, DEFAULT_OUTER_RADIUS);
        Self {
            max_markers: self.max_markers.clamp(1, MAX_MARKERS),
            show_distance: Self::clamped(
                self.show_distance,
                MIN_SHOW_DISTANCE,
                MAX_SHOW_DISTANCE,
                DEFAULT_SHOW_DISTANCE,
            ),
            inner_radius,
            outer_radius: outer_radius.max(inner_radius),
            inner_color: Self::color(self.inner_color),
            outer_color: Self::color(self.outer_color),
            spin: Self::clamped(self.spin, -MAX_SPIN, MAX_SPIN, DEFAULT_SPIN),
        }
    }

    fn color(value: [f32; 4]) -> [f32; 4] {
        value.map(|c| Self::clamped(c, 0.0, 1.0, 1.0))
    }

    fn clamped(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
        if value.is_finite() { value.clamp(min, max) } else { fallback }
    }
}
