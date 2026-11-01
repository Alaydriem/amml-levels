use serde::Deserialize;

use crate::config::{BordersConfig, LabelConfig, LightConfig, ScanConfig, SpheresConfig};

const DEFAULT_MEMORY_BUDGET_MB: u32 = 32;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct LevelsConfig {
    pub scan: ScanConfig,
    pub light: LightConfig,
    pub label: LabelConfig,
    pub borders: BordersConfig,
    pub spheres: SpheresConfig,
    pub telemetry: bool,
    // resident memory the overlay may use; telemetry reports use against it
    pub memory_budget_mb: u32,
}

impl Default for LevelsConfig {
    fn default() -> Self {
        Self {
            scan: ScanConfig::default(),
            light: LightConfig::default(),
            label: LabelConfig::default(),
            borders: BordersConfig::default(),
            spheres: SpheresConfig::default(),
            telemetry: false,
            memory_budget_mb: DEFAULT_MEMORY_BUDGET_MB,
        }
    }
}

impl LevelsConfig {
    // returns a copy with every value moved into its usable range
    pub fn sanitized(self) -> Self {
        Self {
            scan: self.scan.sanitized(),
            light: self.light.sanitized(),
            label: self.label.sanitized(),
            borders: self.borders.sanitized(),
            spheres: self.spheres.sanitized(),
            telemetry: self.telemetry,
            memory_budget_mb: self.memory_budget_mb.max(1),
        }
    }
}
