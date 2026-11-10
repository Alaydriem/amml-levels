use std::sync::Arc;

use crate::config::LightConfig;
use crate::structs::light::LightBand;

const MAX_LIGHT: u8 = 15;

pub struct LightClassifier {
    config: LightConfig,
}

impl LightClassifier {
    pub fn new(config: LightConfig) -> Self {
        Self { config }
    }

    pub fn new_shared(config: LightConfig) -> Arc<Self> {
        Arc::new(Self::new(config))
    }

    // the light at the current time of day: the brighter of block light and sky light darkened for the time
    pub fn light(block_light: u8, sky_light: u8, sky_darken: u8) -> u8 {
        block_light.max(sky_light.saturating_sub(sky_darken)).min(MAX_LIGHT)
    }

    pub fn classify(&self, light: u8) -> LightBand {
        if light <= self.config.red_max {
            LightBand::Red
        } else if light <= self.config.yellow_max {
            LightBand::Yellow
        } else {
            LightBand::Green
        }
    }
}
