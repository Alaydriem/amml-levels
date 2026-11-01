// where the WorldDraw lease stands. Held and Suspended are leases the mod owns and gives back with release; Lost is a
// lease the manager took or refused, which only a CapabilityAvailable event lets the mod ask for again
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseState {
    Released,
    Held,
    Suspended,
    Lost,
}
