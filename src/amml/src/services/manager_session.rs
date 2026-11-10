use std::path::Path;

use amm_client::link::{ApiTable, EventDrain, ManagerLink};
use amm_core::errors::LinkError;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amm_core::structs::lease::AcquireOutcome;
use levels_core::config::LevelsConfig;
use levels_core::services::LevelsController;
use levels_core::structs::manager::ManagerSignal;

use crate::types::LeaseState;

// the registration with the mod manager: turns its events into signals for core and keeps the WorldDraw lease
// while the overlay or a marker needs the world draw
pub struct ManagerSession {
    link: ManagerLink,
    // events of one poll, at most one drain buffer, sized once in connect
    pending: Vec<ModEvent>,
    lease: LeaseState,
}

impl ManagerSession {
    pub const DESCRIPTOR: &'static str = include_str!("../../descriptor.hcl");

    // the indexes of the keybinds in the descriptor
    const TOGGLE_ACTION: u16 = 0;
    const BORDERS_ACTION: u16 = 1;
    const SPHERE_DROP_ACTION: u16 = 2;
    const SPHERE_REMOVE_ACTION: u16 = 3;
    const SPHERE_CLEAR_ACTION: u16 = 4;

    // `config_dir` is the mod's own config folder, an absolute path
    pub fn connect(table: ApiTable, config_dir: &Path) -> Result<Self, LinkError> {
        let link = ManagerLink::without_frame(table, Self::DESCRIPTOR, config_dir)?;
        Ok(Self { link, pending: Vec::with_capacity(EventDrain::CAPACITY), lease: LeaseState::Released })
    }

    // the manager's checked config.hcl; None, with a warning, when it cannot be read
    pub fn config(&self) -> Option<LevelsConfig> {
        match self.link.config::<LevelsConfig>() {
            Ok(config) => Some(config),
            Err(error) => {
                curia::warn!("config not read", { error: error.to_string() });
                None
            }
        }
    }

    pub fn drain(&mut self, controller: &mut LevelsController) {
        self.pending.clear();
        self.pending.extend(self.link.events());
        for position in 0..self.pending.len() {
            let event = self.pending[position];
            self.handle(event, controller);
        }
    }

    // keeps the lease while the controller wants a world draw, suspended or not, so the manager can resume it. A lost
    // lease is not asked for again while core hides for it; once core stops hiding (a world leave), it counts as
    // released, so the next wanted world draw acquires before anything draws
    pub fn sync_lease(&mut self, controller: &mut LevelsController) {
        if self.lease == LeaseState::Lost && !controller.is_lease_hidden() {
            self.lease = LeaseState::Released;
        }
        if controller.wants_world_draw() {
            if self.lease == LeaseState::Released {
                self.acquire(controller);
            }
            return;
        }
        if matches!(self.lease, LeaseState::Held | LeaseState::Suspended) {
            self.link.release(Capability::WorldDraw);
            self.lease = LeaseState::Released;
        }
    }

    fn acquire(&mut self, controller: &mut LevelsController) {
        match self.link.acquire(Capability::WorldDraw) {
            Ok(AcquireOutcome::Granted | AcquireOutcome::Preempted) => {
                self.lease = LeaseState::Held;
                controller.signal(ManagerSignal::LeaseRestored);
            }
            Ok(AcquireOutcome::Denied { .. }) => {
                curia::info!("world draw lease denied");
                self.lease = LeaseState::Lost;
                controller.signal(ManagerSignal::LeaseLost);
            }
            Err(error) => {
                curia::warn!("world draw lease request failed", { error: error.to_string() });
                self.lease = LeaseState::Lost;
                controller.signal(ManagerSignal::LeaseLost);
            }
        }
    }

    // a revoked lease and the leases of a disabled mod are already gone
    fn handle(&mut self, event: ModEvent, controller: &mut LevelsController) {
        match event {
            ModEvent::Keybind { action, phase: KeybindPhase::Pressed } => {
                if let Some(signal) = Self::signal_for_action(action) {
                    controller.signal(signal);
                }
            }
            ModEvent::Suspended { capability: Capability::WorldDraw, .. } => {
                self.lease = LeaseState::Suspended;
                controller.signal(ManagerSignal::LeaseSuspended);
            }
            ModEvent::Resumed(Capability::WorldDraw) => {
                self.lease = LeaseState::Held;
                controller.signal(ManagerSignal::LeaseRestored);
            }
            ModEvent::Revoked { capability: Capability::WorldDraw, .. } => {
                self.lease = LeaseState::Lost;
                controller.signal(ManagerSignal::LeaseLost);
            }
            ModEvent::CapabilityAvailable(Capability::WorldDraw) => {
                if self.lease != LeaseState::Lost {
                    return;
                }
                if controller.wants_world_draw() {
                    self.acquire(controller);
                } else {
                    self.lease = LeaseState::Released;
                    controller.signal(ManagerSignal::LeaseRestored);
                }
            }
            ModEvent::Disabled => {
                self.lease = LeaseState::Released;
                controller.signal(ManagerSignal::Disabled);
            }
            ModEvent::SettingsChanged { .. } => {
                if let Some(config) = self.config() {
                    controller.apply_config(config);
                }
            }
            _ => {}
        }
    }

    fn signal_for_action(action: u16) -> Option<ManagerSignal> {
        match action {
            Self::TOGGLE_ACTION => Some(ManagerSignal::TogglePressed),
            Self::BORDERS_ACTION => Some(ManagerSignal::BordersPressed),
            Self::SPHERE_DROP_ACTION => Some(ManagerSignal::SphereDropPressed),
            Self::SPHERE_REMOVE_ACTION => Some(ManagerSignal::SphereRemovePressed),
            Self::SPHERE_CLEAR_ACTION => Some(ManagerSignal::SphereClearPressed),
            _ => None,
        }
    }
}
