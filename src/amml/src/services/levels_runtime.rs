use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError, TryLockError};

use amm_client::link::LinkState;
use amml_api::amml_abi::{ScreenRender, WorldRendered};
use amml_api::{Client, Ctx, WorldRenderedExt};

use crate::services::{
    FrameStatusReader, InfoDrawer, InfoReader, LevelsLog, ManagerSession, Overlay, ScanDriver, WorldDrawer,
};
use crate::types::{LevelsState, LinkStage};

// what the listeners share: the overlay, the scan and the world drawer behind one lock. The client tick takes the lock
// and waits; the render listener takes it with `try_lock`, so a frame is skipped when the tick holds it and the draw
// never waits. The HUD listener also takes it with `try_lock`
pub struct LevelsRuntime {
    log: LevelsLog,
    state: Mutex<LevelsState>,
    // every `WorldRendered` seen, a frame skipped by the lock included; the panel's frame rate reads it
    frames: AtomicU64,
}

impl LevelsRuntime {
    // seconds between two `world drawn` lines, as long as telemetry is on
    const DRAWN_LINE_SECONDS: f64 = 2.0;

    pub fn new(log: LevelsLog, ctx: Ctx) -> Self {
        Self {
            log,
            state: Mutex::new(LevelsState {
                stage: LinkStage::Off,
                scan: ScanDriver::new(log),
                drawer: WorldDrawer::new(log),
                info: InfoReader::new(ctx),
                hud: InfoDrawer::new(),
                info_shown: false,
                in_world: false,
                scan_logged: false,
                sample_us: 0.0,
                last_vibrant: None,
                drawn_logged_at: None,
            }),
            frames: AtomicU64::new(0),
        }
    }

    // enable: the overlay starts when the manager has connected
    pub fn attach(&self, session: ManagerSession) {
        let mut state = self.locked();
        state.stage = LinkStage::Waiting(session);
        state.in_world = false;
        state.scan_logged = false;
        state.info_shown = false;
    }

    // disable: the overlay stops; dropping the session gives the lease back and closes the link. The loader frees the
    // meshes of a disabled mod
    pub fn detach(&self) {
        let mut state = self.locked();
        state.stage = LinkStage::Off;
        state.drawer.leave();
    }

    // once per client tick on the game thread
    pub fn tick(&self, client: &Client<'_>) {
        let mut guard = self.locked();
        let state = &mut *guard;
        self.advance(state);
        let LinkStage::Running(overlay) = &mut state.stage else { return };
        let report = state.scan.tick(client, overlay);
        state.sample_us = report.sample_us;
        if report.surfaces > 0 && !state.scan_logged {
            state.scan_logged = true;
            self.log.scan(report.columns, report.surfaces);
        }
        if !overlay.is_on() {
            state.scan_logged = false;
        }
        let now = FrameStatusReader::now_seconds();
        let panel = overlay.info_mut();
        panel.frames(self.frames.load(Ordering::Relaxed), now);
        if panel.is_shown() != state.info_shown {
            state.info_shown = panel.is_shown();
            if state.info_shown {
                self.log.info_shown(panel.mode());
            } else {
                self.log.info_hidden();
            }
        }
        let scanned = state.info.tick(client, panel, now);
        if scanned
            && overlay.is_telemetry_on()
            && let Some(counts) = overlay.controller().info().entity_counts()
        {
            self.log.info_entities(counts.total, counts.hostile, counts.passive);
        }
        let in_world = overlay.is_in_world();
        if state.in_world && !in_world {
            state.drawer.leave();
        }
        state.in_world = in_world;
    }

    // after the game's level pass: draws the plan of this frame
    pub fn render(&self, event: &WorldRendered) {
        self.frames.fetch_add(1, Ordering::Relaxed);
        // a busy lock skips the frame; a lock poisoned by a panic in an earlier call is taken as it is, as `locked` does
        let mut guard = match self.state.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return,
        };
        let LevelsState { stage, drawer, info, sample_us, last_vibrant, drawn_logged_at, .. } = &mut *guard;
        let LinkStage::Running(overlay) = stage else { return };
        if !overlay.is_in_world() {
            return;
        }
        let Some(canvas) = event.canvas() else { return };
        let start = FrameStatusReader::now_seconds();
        if *last_vibrant != Some(event.vibrant()) {
            *last_vibrant = Some(event.vibrant());
            self.log.graphics_mode(event.vibrant());
        }
        overlay.set_vibrant(event.vibrant());
        let stats = drawer.render(&canvas, overlay.world_plan());
        let total_us = ((FrameStatusReader::now_seconds() - start) * 1_000_000.0) as f32;
        overlay.timing(*sample_us, total_us, drawer.resident_bytes() + info.resident_bytes());
        overlay.world_timing(stats.build_us, stats.draw_us, stats.meshes_built, stats.quads_converted);
        overlay.info_mut().frame_cost(total_us);
        if overlay.is_telemetry_on() && stats.meshes_drawn > 0 {
            let now = FrameStatusReader::now_seconds();
            if drawn_logged_at.is_none_or(|last| now - last >= Self::DRAWN_LINE_SECONDS) {
                *drawn_logged_at = Some(now);
                self.log.world_drawn(stats.meshes_drawn, event.vibrant());
            }
        }
    }

    // a `ScreenRender` view: the info panel on the HUD. A busy lock skips the view
    pub fn screen(&self, event: &ScreenRender, client: Option<&Client<'_>>, game_thread: bool) {
        if !game_thread {
            return;
        }
        let mut guard = match self.state.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return,
        };
        let LevelsState { stage, hud, .. } = &mut *guard;
        let LinkStage::Running(overlay) = stage else { return };
        let top_screen = client.and_then(Client::top_screen);
        hud.draw(event, top_screen, game_thread, overlay.controller().info(), self.frames.load(Ordering::Relaxed));
    }

    // the `WorldRendered` events seen so far
    pub fn frames(&self) -> u64 {
        self.frames.load(Ordering::Relaxed)
    }

    // a waiting session becomes the overlay once the manager has connected; a manager that is not loaded switches the
    // overlay off for this enable
    fn advance(&self, state: &mut LevelsState) {
        let LinkStage::Waiting(session) = &state.stage else { return };
        if session.state() == LinkState::Connected {
            let LinkStage::Waiting(session) = std::mem::replace(&mut state.stage, LinkStage::Off) else { return };
            state.stage = LinkStage::Running(Box::new(Overlay::connect(session)));
            self.log.manager_linked();
        } else if session.manager_missing() {
            state.stage = LinkStage::Off;
            self.log.manager_missing();
        }
    }

    // runs `read` on the state under the lock
    pub fn inspect<R>(&self, read: impl FnOnce(&mut LevelsState) -> R) -> R {
        read(&mut self.locked())
    }

    // a panic poisoned the lock in an earlier call: the state is still whole, so it is taken as it is
    fn locked(&self) -> MutexGuard<'_, LevelsState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
