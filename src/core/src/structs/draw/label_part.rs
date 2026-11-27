// A run of whole labels in the label quad list that the host builds as one mesh. The key is a hash of everything the
// quads of the run depend on, so a run with the same key as before holds the same quads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelPart {
    pub key: u64,
    // the quads of the part are `quads[start..end]` of the label quad list
    pub start: usize,
    pub end: usize,
}
