use std::ffi::c_void;
use std::sync::atomic::AtomicPtr;

use amml_game::{EntryId, EntryNeed, EntryResolver, GameEntries, GameError, HookOrder};
use amml_sdk::amml_abi::AmmlLogLevel;
use amml_sdk::{AmmlMod, Api, HookHandle, ModDirs, SdkError};

use crate::detours::{client_update, render_level};
use crate::game::{BaseActorRenderer, GameMirror};
use crate::services::LevelsRuntime;
use crate::types::{LevelsEntry, MeshCalls};

// the AMML entry point: load notes the folders, enable resolves the entries, installs the hooks and starts the
// overlay, disable removes the hooks and stops it
pub struct LevelsMod;

impl LevelsMod {
    // the mod id as a static string for the host and logger; a test checks it equals AmmlMod::id()
    pub const MODULE: &'static str = "@alaydriem/levels";
    // the entries the overlay cannot run without: the client, the player's pose, dimension and block source, the
    // chunk reads, the light and surface reads, the frame's camera position, and the world mesh path with the level renderer's wireframe material
    pub const REQUIRED: [EntryId; 61] = [
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
        EntryId::CAMERA_FORWARD,
        EntryId::VIEW_RENDER_OBJECT_VIEW_DATA,
        EntryId::VIEW_RENDER_DATA_CAMERA_POS,
        LevelsEntry::SCREEN_CONTEXT_TESSELLATOR,
        LevelsEntry::BASE_ACTOR_RENDERER_CTOR,
        LevelsEntry::BASE_ACTOR_RENDERER_SIZE,
        LevelsEntry::GRAPHICS_MODE_DEFERRED,
        LevelsEntry::RENDER_LEVEL,
        LevelsEntry::FRAME_VIEWS,
        LevelsEntry::TESSELLATOR_BEGIN,
        LevelsEntry::TESSELLATOR_END,
        LevelsEntry::TESSELLATOR_CLEAR,
        LevelsEntry::TESSELLATOR_TESSELLATING,
        LevelsEntry::MESH_DTOR,
        LevelsEntry::MESH_IS_VALID,
        LevelsEntry::MESH_RENDER,
        LevelsEntry::MESH_SIZE,
        LevelsEntry::MATRIX_STACK_PUSH,
        LevelsEntry::MATRIX_STACK_REF_DTOR,
        LevelsEntry::SET_ENTITY_CONSTANTS,
        LevelsEntry::ENTITY_CONSTANT_BUFFER,
        LevelsEntry::GET_TEXTURE_GROUP,
        LevelsEntry::TEXTURE_GROUP_GET_TEXTURE,
        LevelsEntry::TEXTURE_GROUP_UPLOAD_TEXTURE,
        LevelsEntry::TEXTURE_GROUP_UNLOAD_TEXTURE,
        LevelsEntry::RESOURCE_LOCATION_CTOR,
        LevelsEntry::IMAGE_BUFFER_CTOR,
        LevelsEntry::RESOURCE_LOCATION_FULL_HASH,
        LevelsEntry::IMAGE_BUFFER_DESCRIPTION,
        LevelsEntry::IMAGE_DESCRIPTION_DEPTH,
        LevelsEntry::TEXTURE_GROUP_BASE_OFFSET,
        LevelsEntry::SCREEN_CONTEXT_MESH_CONTEXT,
        LevelsEntry::SCREEN_CONTEXT_CAMERA,
        LevelsEntry::SCREEN_CONTEXT_CONSTANT_BUFFERS,
        LevelsEntry::SCREEN_CONTEXT_CURRENT_SHADER_COLOR,
        LevelsEntry::LEVEL_RENDERER_PLAYER,
        LevelsEntry::WIREFRAME_MATERIAL,
        LevelsEntry::MATERIAL_INFO_MATERIAL,
        LevelsEntry::MATERIAL_ENABLE_BLEND,
        LevelsEntry::MATERIAL_DEPTH_WRITE,
        LevelsEntry::MATERIAL_BLEND_SOURCE,
        LevelsEntry::MATERIAL_BLEND_DESTINATION,
    ];

    // every entry the overlay asks for, all required
    pub fn wanted() -> Vec<(EntryId, EntryNeed)> {
        Self::REQUIRED.iter().map(|id| (*id, EntryNeed::Required)).collect()
    }

    // the entries, or an error when a required entry is missing
    pub fn resolve<R: EntryResolver>(resolver: &R) -> Result<GameEntries, GameError> {
        GameEntries::resolve(resolver, &Self::wanted())
    }

    // the required mirrors that do not fit the build: the world meshes cannot be drawn with them
    pub fn layout_faults(entries: &GameEntries) -> Vec<String> {
        let mesh = MeshCalls::misfit(entries).map(|fault| fault.to_string());
        mesh.into_iter().chain(BaseActorRenderer::misfit(entries)).collect()
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

    // both hooks only read the game after its own work, so both run after every writer (a camera mod runs before
    // them). The render-level hook draws after the game's level pass
    fn install_hooks(api: &Api, runtime: &LevelsRuntime, entries: &GameEntries) -> Result<(), SdkError> {
        let missing = |id: EntryId| SdkError::MissingEntry(id.as_str().to_string());
        let update =
            entries.get(EntryId::CLIENT_INSTANCE_UPDATE).ok_or_else(|| missing(EntryId::CLIENT_INSTANCE_UPDATE))?;
        let render = entries.get(LevelsEntry::RENDER_LEVEL).ok_or_else(|| missing(LevelsEntry::RENDER_LEVEL))?;
        let read = HookOrder::READ_AFTER_WRITERS;
        Self::hook(
            api,
            runtime,
            update,
            read,
            client_update::detour as *const c_void,
            client_update::GUARD.original(),
        )?;
        Self::hook(api, runtime, render, read, render_level::detour as *const c_void, render_level::GUARD.original())
            .map(|_| ())
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
        if let Err(error) = Self::install_hooks(api, runtime, &entries) {
            Self::remove_hooks(api, runtime);
            return Err(error);
        }
        runtime.start(entries);
        Ok(())
    }

    // the hooks go first, so no new frame enters the overlay; then the registration ends
    fn disable(api: &Api) {
        let Some(runtime) = LevelsRuntime::get() else { return };
        runtime.latch().raise();
        Self::remove_hooks(api, runtime);
        runtime.stop();
    }
}
