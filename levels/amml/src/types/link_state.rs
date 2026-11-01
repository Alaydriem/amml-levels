// where the registration with the mod manager stands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    // enable did not reach the manager yet; the first tick tries again
    Waiting,
    Attached,
    // disabled, or the manager refused or is missing
    Off,
}
