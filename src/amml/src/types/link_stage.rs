use crate::services::{ManagerSession, Overlay};

// where the registration with the mod manager stands
pub enum LinkStage {
    // enable started the link; the manager has not connected yet
    Waiting(ManagerSession),
    // the manager is connected and the overlay runs
    Running(Box<Overlay>),
    // disabled, or the manager is missing
    Off,
}
