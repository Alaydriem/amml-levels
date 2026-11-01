// what a mod manager event means for the overlay
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagerSignal {
    // the toggle keybind was pressed
    TogglePressed,
    // the chunk border keybind was pressed
    BordersPressed,
    // the keybind that drops a spawn sphere at the feet of the player was pressed
    SphereDropPressed,
    // the keybind that removes the nearest spawn sphere was pressed
    SphereRemovePressed,
    // the keybind that removes every spawn sphere was pressed
    SphereClearPressed,
    // the WorldDraw lease was denied or revoked
    LeaseLost,
    // the WorldDraw lease was suspended for a while
    LeaseSuspended,
    // a suspended WorldDraw lease is back
    LeaseRestored,
    // the player disabled the mod in the manager
    Disabled,
}
