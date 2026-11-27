#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EntityCounts {
    pub total: u16,
    pub players: u16,
    pub hostile: u16,
    pub passive: u16,
    pub other: u16,
    // (kind byte, count), most common first; a count of 0 is an empty slot
    pub top_hostile: [(u8, u16); 3],
}
