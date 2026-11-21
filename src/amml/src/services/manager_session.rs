use amm_client::link::{EventDrain, LinkState, ManagerLink};
use amm_core::errors::LinkError;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amm_core::structs::lease::AcquireOutcome;
use amml_api::Ctx;
use levels_core::config::LevelsConfig;
use levels_core::services::LevelsController;
use levels_core::structs::manager::ManagerSignal;

use crate::services::LevelsLog;
use crate::types::LeaseState;

// the registration with the mod manager: turns its events into signals for core and keeps the WorldDraw lease
// while the overlay or a marker needs the world draw
pub struct ManagerSession {
    link: ManagerLink,
    // events of one poll, at most one drain buffer, sized once in connect
    pending: Vec<ModEvent>,
    lease: LeaseState,
    log: LevelsLog,
}

impl ManagerSession {
    pub const DESCRIPTOR: &'static str = include_str!("../../descriptor.hcl");

    // the indexes of the keybinds in the descriptor
    const TOGGLE_ACTION: u16 = 0;
    const BORDERS_ACTION: u16 = 1;
    const SPHERE_DROP_ACTION: u16 = 2;
    const SPHERE_REMOVE_ACTION: u16 = 3;
    const SPHERE_CLEAR_ACTION: u16 = 4;

    // the link waits for the manager's tables; `state` says when it is connected
    pub fn start(ctx: &Ctx) -> Result<Self, LinkError> {
        let link = ManagerLink::start_without_frame(ctx, Self::DESCRIPTOR)?;
        Ok(Self {
            link,
            pending: Vec::with_capacity(EventDrain::CAPACITY),
            lease: LeaseState::Released,
            log: LevelsLog::new(ctx.log()),
        })
    }

    pub fn state(&self) -> LinkState {
        self.link.state()
    }

    // true once every mod has loaded and none was the manager
    pub fn manager_missing(&self) -> bool {
        matches!(self.link.config::<LevelsConfig>(), Err(LinkError::NoManager))
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
            self.log.lease_released();
        }
    }

    fn acquire(&mut self, controller: &mut LevelsController) {
        match self.link.acquire(Capability::WorldDraw) {
            Ok(AcquireOutcome::Granted | AcquireOutcome::Preempted) => {
                self.lease = LeaseState::Held;
                self.log.lease_held();
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
                    let before = controller.marker_count();
                    controller.signal(signal);
                    self.log_spheres(signal, before, controller.marker_count());
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
            ModEvent::Disabled | ModEvent::ManagerGone => {
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

    // one line for a sphere key that changed the spheres
    fn log_spheres(&self, signal: ManagerSignal, before: usize, after: usize) {
        match signal {
            ManagerSignal::SphereDropPressed if after > before => self.log.sphere_dropped(after),
            ManagerSignal::SphereRemovePressed if after < before => self.log.sphere_removed(after),
            ManagerSignal::SphereClearPressed if before > 0 && after == 0 => self.log.spheres_cleared(),
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

impl Drop for ManagerSession {
    // the lease goes back before the link closes
    fn drop(&mut self) {
        if matches!(self.lease, LeaseState::Held | LeaseState::Suspended) {
            self.link.release(Capability::WorldDraw);
            self.log.lease_released();
        }
    }
}
