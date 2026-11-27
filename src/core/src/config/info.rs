use serde::Deserialize;

use crate::structs::info::{InfoCorner, InfoMode};

pub const MAX_ENTITY_RADIUS: f32 = 64.0;
const MIN_ENTITY_RADIUS: f32 = 4.0;
const DEFAULT_ENTITY_RADIUS: f32 = 32.0;
const MIN_REFRESH_SECONDS: f32 = 0.05;
const MAX_REFRESH_SECONDS: f32 = 2.0;
const DEFAULT_REFRESH_SECONDS: f32 = 0.25;
const MIN_ENTITY_SCAN_SECONDS: f32 = 0.1;
const MAX_ENTITY_SCAN_SECONDS: f32 = 5.0;
const DEFAULT_ENTITY_SCAN_SECONDS: f32 = 0.5;
const DEFAULT_BACKGROUND_ALPHA: f32 = 0.5;
const MAX_TOP_OFFSET: f32 = 200.0;
// clears the game's coordinates text and the days-played line under it
const DEFAULT_TOP_OFFSET: f32 = 24.0;

// the F3+T panel; each `show_*` turns one group of lines on or off
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct InfoConfig {
    pub mode: InfoMode,
    pub corner: InfoCorner,
    pub refresh_seconds: f32,
    // blocks around the player counted on the entity lines
    pub entity_radius: f32,
    pub entity_scan_seconds: f32,
    pub background_alpha: f32,
    // HUD units between the top edge of the screen and the panel
    pub top_offset: f32,
    pub show_position: bool,
    pub show_world: bool,
    pub show_chunk: bool,
    pub show_light: bool,
    pub show_target: bool,
    pub show_motion: bool,
    pub show_entities: bool,
    pub show_performance: bool,
    pub show_amml: bool,
}

impl Default for InfoConfig {
    fn default() -> Self {
        Self {
            mode: InfoMode::Toggle,
            corner: InfoCorner::TopLeft,
            refresh_seconds: DEFAULT_REFRESH_SECONDS,
            entity_radius: DEFAULT_ENTITY_RADIUS,
            entity_scan_seconds: DEFAULT_ENTITY_SCAN_SECONDS,
            background_alpha: DEFAULT_BACKGROUND_ALPHA,
            top_offset: DEFAULT_TOP_OFFSET,
            show_position: true,
            show_world: true,
            show_chunk: true,
            show_light: true,
            show_target: true,
            show_motion: true,
            show_entities: true,
            show_performance: true,
            show_amml: true,
        }
    }
}

impl InfoConfig {
    pub fn sanitized(self) -> Self {
        Self {
            refresh_seconds: Self::clamped(
                self.refresh_seconds,
                MIN_REFRESH_SECONDS,
                MAX_REFRESH_SECONDS,
                DEFAULT_REFRESH_SECONDS,
            ),
            entity_radius: Self::clamped(
                self.entity_radius,
                MIN_ENTITY_RADIUS,
                MAX_ENTITY_RADIUS,
                DEFAULT_ENTITY_RADIUS,
            ),
            entity_scan_seconds: Self::clamped(
                self.entity_scan_seconds,
                MIN_ENTITY_SCAN_SECONDS,
                MAX_ENTITY_SCAN_SECONDS,
                DEFAULT_ENTITY_SCAN_SECONDS,
            ),
            background_alpha: Self::clamped(self.background_alpha, 0.0, 1.0, DEFAULT_BACKGROUND_ALPHA),
            top_offset: Self::clamped(self.top_offset, 0.0, MAX_TOP_OFFSET, DEFAULT_TOP_OFFSET),
            ..self
        }
    }

    fn clamped(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
        if value.is_finite() { value.clamp(min, max) } else { fallback }
    }
}
