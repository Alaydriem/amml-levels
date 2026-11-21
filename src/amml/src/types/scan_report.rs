// what one tick of the scan did: the microseconds of the sampling, the columns sampled and the surfaces found
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ScanReport {
    pub sample_us: f32,
    pub columns: usize,
    pub surfaces: usize,
}
