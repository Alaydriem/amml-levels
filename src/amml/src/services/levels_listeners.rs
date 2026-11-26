use std::sync::Arc;

use amml_api::amml_abi::{AmmlThread, ClientTick, ScreenRender, WorldRendered};
use amml_api::{ApiError, Ctx, HookOrder, Listener};

use crate::services::LevelsRuntime;

// the loader listeners of the overlay; each only hands its event to the runtime. Dropping them removes them. All
// run after every writer: they only read the game after its own work. The HUD listener starts only on a loader that
// raises `ScreenRender`
pub struct LevelsListeners {
    listeners: Vec<Listener>,
    runtime: Arc<LevelsRuntime>,
}

impl LevelsListeners {
    pub fn start(ctx: &Ctx, runtime: Arc<LevelsRuntime>) -> Result<Self, ApiError> {
        let events = ctx.events();
        let mut listeners = Vec::with_capacity(3);
        let ticking = Arc::clone(&runtime);
        listeners.push(events.listen::<ClientTick>(HookOrder::READ_AFTER_WRITERS, move |_, at| {
            if let Some(client) = at.client() {
                ticking.tick(&client);
            }
        })?);
        let drawing = Arc::clone(&runtime);
        listeners.push(
            events.listen::<WorldRendered>(HookOrder::READ_AFTER_WRITERS, move |event, _| drawing.render(event))?,
        );
        if ctx.available::<ScreenRender>() {
            let hud = Arc::clone(&runtime);
            listeners.push(events.listen::<ScreenRender>(HookOrder::READ_AFTER_WRITERS, move |event, at| {
                hud.screen(event, at.client().as_ref(), at.thread == AmmlThread::GAME);
            })?);
        }
        Ok(Self { listeners, runtime })
    }

    // the listeners go first, so no listener runs while the runtime lets go of the link
    pub fn stop(self) {
        let Self { listeners, runtime } = self;
        drop(listeners);
        runtime.detach();
    }
}
