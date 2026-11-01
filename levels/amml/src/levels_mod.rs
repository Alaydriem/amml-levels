use std::ffi::c_void;
use std::sync::atomic::AtomicPtr;

use amml_game::{EntryId, EntryNeed, EntryResolver, GameEntries, GameError, HookOrder};
use amml_sdk::amml_abi::AmmlLogLevel;
use amml_sdk::{AmmlMod, Api, HookHandle, ModDirs, SdkError};

use crate::detours::{client_update, extract_name_tags, render_level, setup_camera, view_object};
use crate::game::{BaseActorRenderer, GameMirror, LevelRendererPlayer, NameTagRenderObject};
use crate::services::LevelsRuntime;
use crate::types::{LevelsEntry, LevelsFeature};

// the AMML entry point: load notes the folders, enable resolves the entries, installs the hooks and starts the
// overlay, disable removes the hooks and stops it
pub struct LevelsMod;

impl LevelsMod {
    // the mod id as a static string for the host and logger; a test checks it equals AmmlMod::id()
    pub const MODULE: &'static str = "@alaydriem/levels";
    // the entries the overlay cannot run without: the client, the player's pose, dimension and block source, the
    // chunk reads, the light and surface reads, the render camera and the name-tag draw path
    pub const REQUIRED: [EntryId; 31] = [
        EntryId::CLIENT_INSTANCE_UPDATE,
        EntryId::CLIENT_INSTANCE_GET_LOCAL_PLAYER,
        EntryId::CLIENT_INSTANCE_IS_WORLD_ACTIVE,
        EntryId::CLIENT_INSTANCE_IS_SHOWING_MENU,
        EntryId::CLIENT_INSTANCE_GET_OPTIONS,
        EntryId::I_OPTION_REGISTRY_GET_HIDE_HUD,
        EntryId::ACTOR_STATE_VECTOR_COMPONENT,
        EntryId::ACTOR_ACTOR_ROTATION_COMPONENT,
        EntryId::STATE_VECTOR_COMPONENT_POS,
        EntryId::ACTOR_ROTATION_COMPONENT_ROT,
        EntryId::ACTOR_GET_DIMENSION,
        EntryId::DIMENSION_GET_DIMENSION_ID,
        EntryId::ACTOR_GET_DIMENSION_BLOCK_SOURCE,
        EntryId::BLOCK_SOURCE_GET_CHUNK,
        EntryId::LEVEL_CHUNK_SUB_CHUNKS,
        EntryId::LEVEL_CHUNK_GET_BLOCK,
        EntryId::BLOCK_SOURCE_GET_BRIGHTNESS_PAIR,
        EntryId::BLOCK_CAN_PROVIDE_FULL_SUPPORT,
        EntryId::BLOCK_TYPE_GET_COLLISION_SHAPE,
        EntryId::LEVEL_RENDERER_PLAYER_SETUP_CAMERA,
        EntryId::LEVEL_RENDERER_PLAYER_CREATE_VIEW_RENDER_OBJECT,
        EntryId::CAMERA_FORWARD,
        EntryId::VIEW_RENDER_OBJECT_VIEW_DATA,
        EntryId::VIEW_RENDER_DATA_CAMERA_POS,
        LevelsEntry::LEVEL_RENDERER_PLAYER_VTABLE,
        LevelsEntry::EXTRACT_NAME_TAGS,
        LevelsEntry::NAME_TAG_SIZE,
        LevelsEntry::SCREEN_CONTEXT_TESSELLATOR,
        LevelsEntry::BASE_ACTOR_RENDERER_CTOR,
        LevelsEntry::BASE_ACTOR_RENDERER_SIZE,
        LevelsEntry::EXTRACT_RENDER_TEXT_OBJECT,
    ];

    // every entry the overlay asks for: the required ones, then each optional feature's
    pub fn wanted() -> Vec<(EntryId, EntryNeed)> {
        let required = Self::REQUIRED.iter().map(|id| (*id, EntryNeed::Required));
        let optional = LevelsFeature::ALL.iter().flat_map(|feature| {
            LevelsFeature::entries(feature).iter().map(|id| (*id, EntryNeed::Optional { feature }))
        });
        required.chain(optional).collect()
    }

    // the entries, or an error when a required entry is missing
    pub fn resolve<R: EntryResolver>(resolver: &R) -> Result<GameEntries, GameError> {
        GameEntries::resolve(resolver, &Self::wanted())
    }

    // the required mirrors that do not fit the build: the labels cannot be drawn with them
    pub fn layout_faults(entries: &GameEntries) -> Vec<String> {
        [BaseActorRenderer::misfit(entries), NameTagRenderObject::misfit(entries)].into_iter().flatten().collect()
    }

    // one line per feature that is off, naming it and the entry or the mirror that turned it off
    pub fn off_lines(entries: &GameEntries) -> Vec<String> {
        let mut causes = entries.off_features();
        for feature in LevelsFeature::ALL {
            if causes.iter().all(|(off, _)| *off != feature)
                && let Some(cause) = LevelsFeature::misfit(entries, feature)
            {
                causes.push((feature, cause));
            }
        }
        causes
            .into_iter()
            .map(|(feature, cause)| match feature {
                LevelsFeature::FONT => format!("{feature} is off ({cause}); the labels use a fixed width table"),
                LevelsFeature::MARKERS => {
                    format!("{feature} is off ({cause}); chunk borders and spawn spheres are not drawn")
                }
                _ => format!("{feature} is off ({cause})"),
            })
            .collect()
    }

    #[allow(unsafe_code)]
    fn hook(
        api: &Api,
        runtime: &LevelsRuntime,
        target: u64,
        priority: i32,
        detour: *const c_void,
        original: &'static AtomicPtr<c_void>,
    ) -> Result<HookHandle, SdkError> {
        // SAFETY: `target` is the derived start of the game function each caller pairs with the detour written for
        // its signature (this in rcx, then the C++ parameters as the game passes them); each original slot belongs
        // to one hook only
        let hook = unsafe { api.hook(target, detour, priority, original) }?;
        if runtime.keep_hook(hook) {
            return Ok(hook);
        }
        let _ = api.unhook(hook);
        Err(SdkError::Mod(format!("more than {} hooks", LevelsRuntime::HOOK_CAPACITY)))
    }

    fn remove_hooks(api: &Api, runtime: &LevelsRuntime) {
        for hook in runtime.take_hooks() {
            if let Err(error) = api.unhook(hook) {
                api.log(Self::MODULE, AmmlLogLevel::WARN, &format!("unhook failed: {error}"));
            }
        }
    }

    // the readers run after every writer (a camera mod runs before them); the name-tag hook changes the game's
    // list, so it is a writer. The render-level hook only draws after the game, so it is a reader, installed only
    // when the markers feature is on; when it alone fails, markers is off and the light numbers keep working
    fn install_hooks(api: &Api, runtime: &LevelsRuntime, entries: &GameEntries) -> Result<(), SdkError> {
        let missing = |id: EntryId| SdkError::MissingEntry(id.as_str().to_string());
        let update =
            entries.get(EntryId::CLIENT_INSTANCE_UPDATE).ok_or_else(|| missing(EntryId::CLIENT_INSTANCE_UPDATE))?;
        let setup = entries
            .get(EntryId::LEVEL_RENDERER_PLAYER_SETUP_CAMERA)
            .ok_or_else(|| missing(EntryId::LEVEL_RENDERER_PLAYER_SETUP_CAMERA))?;
        let view = entries
            .get(EntryId::LEVEL_RENDERER_PLAYER_CREATE_VIEW_RENDER_OBJECT)
            .ok_or_else(|| missing(EntryId::LEVEL_RENDERER_PLAYER_CREATE_VIEW_RENDER_OBJECT))?;
        let extract = LevelRendererPlayer::extract_name_tags_target(entries)
            .ok_or_else(|| missing(LevelsEntry::EXTRACT_NAME_TAGS))?;
        let read = HookOrder::READ_AFTER_WRITERS;
        Self::hook(
            api,
            runtime,
            update,
            read,
            client_update::detour as *const c_void,
            client_update::GUARD.original(),
        )?;
        Self::hook(api, runtime, setup, read, setup_camera::detour as *const c_void, setup_camera::GUARD.original())?;
        Self::hook(api, runtime, view, read, view_object::detour as *const c_void, view_object::GUARD.original())?;
        Self::hook(
            api,
            runtime,
            extract,
            HookOrder::WRITE,
            extract_name_tags::detour as *const c_void,
            extract_name_tags::GUARD.original(),
        )?;
        if LevelsFeature::is_on(entries, LevelsFeature::MARKERS) {
            let render = entries.get(LevelsEntry::RENDER_LEVEL).ok_or_else(|| missing(LevelsEntry::RENDER_LEVEL))?;
            let hooked = Self::hook(
                api,
                runtime,
                render,
                read,
                render_level::detour as *const c_void,
                render_level::GUARD.original(),
            );
            if let Err(error) = hooked {
                let line = format!(
                    "{} is off (the render-level hook failed: {error}); chunk borders and spawn spheres are not drawn",
                    LevelsFeature::MARKERS
                );
                api.log(Self::MODULE, AmmlLogLevel::WARN, &line);
            }
        }
        Ok(())
    }
}

impl AmmlMod for LevelsMod {
    const NAMESPACE: &'static str = "alaydriem";
    const NAME: &'static str = "levels";

    // runs under the loader lock: only the folders are noted, nothing starts a thread
    fn load(api: &Api) -> Result<(), SdkError> {
        let dirs = ModDirs::new(api, Self::MODULE)?;
        LevelsRuntime::install(&dirs.config_dir);
        Ok(())
    }

    fn enable(api: &Api) -> Result<(), SdkError> {
        let runtime = LevelsRuntime::get().ok_or_else(|| SdkError::Mod("load did not run".to_string()))?;
        let entries = Self::resolve(api).map_err(|error| SdkError::Mod(error.to_string()))?;
        let faults = Self::layout_faults(&entries);
        if !faults.is_empty() {
            return Err(SdkError::Mod(faults.join("; ")));
        }
        let off_lines = Self::off_lines(&entries);
        for line in &off_lines {
            api.log(Self::MODULE, AmmlLogLevel::WARN, line);
        }
        if off_lines.is_empty() {
            api.log(Self::MODULE, AmmlLogLevel::INFO, "features off: none");
        }
        if let Err(error) = Self::install_hooks(api, runtime, &entries) {
            Self::remove_hooks(api, runtime);
            return Err(error);
        }
        runtime.start(entries);
        Ok(())
    }

    // the hooks go first, so no new frame enters the overlay; then the registration ends. The ring stays allocated
    fn disable(api: &Api) {
        let Some(runtime) = LevelsRuntime::get() else { return };
        runtime.latch().raise();
        Self::remove_hooks(api, runtime);
        runtime.stop();
    }
}
