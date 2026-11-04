use serde::Deserialize;

use crate::config::{BordersConfig, LabelConfig, LightConfig, ScanConfig, SpheresConfig};
use crate::services::LabelMeshBuilder;
use crate::structs::draw::ColorQuad;

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
        let memory_budget_mb = self.memory_budget_mb.max(1);
        let mut scan = self.scan.sanitized();
        // the label mesh reserves its quads up front; keep that reservation within half the budget
        let bytes_per_label = LabelMeshBuilder::MAX_QUADS_PER_LABEL * std::mem::size_of::<ColorQuad>();
        let half_budget_bytes = memory_budget_mb as usize * 1024 * 1024 / 2;
        let label_bound = (half_budget_bytes / bytes_per_label).clamp(1, u32::MAX as usize) as u32;
        scan.max_labels = scan.max_labels.min(label_bound);
        Self {
            scan,
            light: self.light.sanitized(),
            label: self.label.sanitized(),
            borders: self.borders.sanitized(),
            spheres: self.spheres.sanitized(),
            telemetry: self.telemetry,
            memory_budget_mb,
        }
    }
}
