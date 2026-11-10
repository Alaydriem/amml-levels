use serde::Deserialize;

const MAX_LIGHT: u8 = 15;

// a light value at or below red_max is red, at or below yellow_max is yellow, above it is green
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default)]
pub struct LightConfig {
    pub red_max: u8,
    pub yellow_max: u8,
}

impl Default for LightConfig {
    fn default() -> Self {
        Self { red_max: 0, yellow_max: 7 }
    }
}

impl LightConfig {
    pub fn sanitized(self) -> Self {
        let red_max = self.red_max.min(MAX_LIGHT);
        Self { red_max, yellow_max: self.yellow_max.clamp(red_max, MAX_LIGHT) }
    }
}
