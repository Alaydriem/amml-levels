use std::sync::{Arc, Mutex, PoisonError};

use amml_api::amml_abi::{
    ClientTick, InjectedInputReleased, InputLeaseLost, InputLeaseResumed, InputLeaseSuspended, ModsLoaded,
    ProviderGone, ProviderReady, ScreenRender, ScreenshotTaken, ScreenshotWritten, WorldRendered,
};
use amml_api::{AmmlMod, ApiError, Area, Ctx, DisableCtx, Uses};

use crate::services::{LevelsListeners, LevelsLog, LevelsRuntime, ManagerSession};

// what the enabled mod owns. `disable` takes it, which drops the listeners and closes the link
static RUNNING: Mutex<Option<LevelsListeners>> = Mutex::new(None);

// the AMML entry point: enable starts the manager link and the listeners, disable removes them and leaves the mod
// manager
pub struct LevelsMod;

impl AmmlMod for LevelsMod {
    const NAMESPACE: &'static str = "alaydriem";
    const NAME: &'static str = "levels";
    const USES: Uses = amml_api::amml_uses! {
        feature "overlay" {
            required ClientTick;
            required WorldRendered;
            required ProviderReady;
            required ProviderGone;
            required ModsLoaded;
            required InputLeaseSuspended;
            required InputLeaseResumed;
            required InputLeaseLost;
            required InjectedInputReleased;
            required ScreenshotTaken;
            required ScreenshotWritten;
            required area Area::INPUT_LEASE;
            required area Area::CLIENT_READ;
            required area Area::CLIENT_HUD;
            required area Area::ACTOR_READ.at(3);
            required area Area::WORLD_SKY_DARKEN;
            required area Area::WORLD_SCAN_SURFACES;
            required area Area::RENDER_WORLD_CANVAS.at(2);
        }
        feature "info" {
            optional ScreenRender;
            optional area Area::UI_SCREEN;
            optional area Area::UI_DRAW;
            optional area Area::UI_TEXT;
            optional area Area::CLIENT_TOP_SCREEN;
            optional area Area::WORLD_CHUNK_READ;
            optional area Area::WORLD_BIOMES.at(2);
            optional area Area::WORLD_BLOCKS;
            optional area Area::PLAYER_HIT_RESULT;
            optional area Area::CLIENT_WORLD_INFO;
            optional area Area::WORLD_ACTORS_IN.at(2);
        }
    };

    fn enable(ctx: &Ctx) -> Result<(), ApiError> {
        let log = ctx.log();
        let session = ManagerSession::start(ctx).map_err(|error| {
            log.error("levels: off", &[("error", &error.to_string())]);
            ApiError::Refused
        })?;
        let runtime = Arc::new(LevelsRuntime::new(LevelsLog::new(log), *ctx));
        runtime.attach(session);
        let listeners = LevelsListeners::start(ctx, runtime).inspect_err(|error| {
            log.error("levels: off", &[("error", &error.to_string())]);
        })?;
        *RUNNING.lock().unwrap_or_else(PoisonError::into_inner) = Some(listeners);
        Ok(())
    }

    fn disable(_ctx: &Ctx, _how: &DisableCtx) {
        let running = RUNNING.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(listeners) = running {
            listeners.stop();
        }
    }
}
