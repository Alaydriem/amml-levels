use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::{Mutex, OnceLock, PoisonError};

use amm_client::link::ApiTable;
use amm_core::errors::LinkError;
use amml_game::{ClientInstance, GameEntries, HeldClient, StopLatch};
use amml_sdk::{HookHandle, ModRuntime, PanicText};

use crate::game::{FrameRenderObject, GraphicsMode, WireframeMaterial};
use crate::services::{
    FrameStatusReader, GameMeshFactory, MaterialPatch, MeshFactory, NameTagMaterial, Overlay, ScanDriver, WorldDrawer,
};
use crate::types::{LinkState, MeshCalls, WorldFrame};

// what the detours share: the folders, the stop latch and the state of the current enable. The tick and the
// render-level hook may run on more than one thread; each takes the state lock, the render hook with `try_lock`, so a
// draw frame never waits. Disable may run on another thread and waits for the lock
pub struct LevelsRuntime {
    config_dir: PathBuf,
    latch: StopLatch,
    entries: OnceLock<GameEntries>,
    // the world mesh path; set at the first enable
    mesh_calls: OnceLock<MeshCalls>,
    client: HeldClient,
    state: Mutex<RuntimeState>,
}

// everything behind the lock. The world drawer's meshes and texture belong to the render device, so a disable does not free them: `epoch` moves
// on each enable and on each world leave the tick sees, and the next render frame whose `world_epoch` is behind it
// drops them first, on the render thread
pub struct RuntimeState {
    overlay: Option<Overlay>,
    link: LinkState,
    scan: ScanDriver,
    world: WorldDrawer,
    material: MaterialPatch,
    name_tag: NameTagMaterial,
    // the graphics mode the last drawn frame used, to log a change once
    last_vibrant: Option<bool>,
    epoch: u64,
    world_epoch: u64,
    // whether the last tick saw a world loaded
    in_world: bool,
    hooks: Vec<HookHandle>,
    sample_us: f32,
}

// SAFETY: the raw pointers in the state are blocks of the game heap and addresses of game objects. They are used by
// the tick and the render-level hook, only under the state lock, and nowhere else
#[allow(unsafe_code)]
unsafe impl Send for RuntimeState {}

static RUNTIME: OnceLock<LevelsRuntime> = OnceLock::new();

impl LevelsRuntime {
    // the detours: client update, render level
    pub const HOOK_CAPACITY: usize = 2;

    pub fn new(config_dir: &Path) -> Self {
        Self {
            config_dir: config_dir.to_path_buf(),
            latch: StopLatch::new(),
            entries: OnceLock::new(),
            mesh_calls: OnceLock::new(),
            client: HeldClient::new(),
            state: Mutex::new(RuntimeState {
                overlay: None,
                link: LinkState::Off,
                scan: ScanDriver::new(),
                world: WorldDrawer::new(MeshFactory::Game(GameMeshFactory::new(Self::allocate))),
                material: MaterialPatch::new(),
                name_tag: NameTagMaterial::new(),
                last_vibrant: None,
                epoch: 0,
                world_epoch: 0,
                in_world: false,
                hooks: Vec::with_capacity(Self::HOOK_CAPACITY),
                sample_us: 0.0,
            }),
        }
    }

    // the runtime of this DLL; the first call wins
    pub fn install(config_dir: &Path) -> &'static LevelsRuntime {
        RUNTIME.get_or_init(|| Self::new(config_dir))
    }

    pub fn get() -> Option<&'static LevelsRuntime> {
        RUNTIME.get()
    }

    // raised by disable and by a panic in a hook body
    pub fn latch(&self) -> &StopLatch {
        &self.latch
    }

    pub fn entries(&self) -> Option<&GameEntries> {
        self.entries.get()
    }

    // the game heap's allocator, as a plain function for the mesh factory
    #[allow(unsafe_code)]
    unsafe extern "C" fn allocate(size: usize) -> *mut u8 {
        ModRuntime::get().map_or(std::ptr::null_mut(), |runtime| runtime.api().heap_new(size))
    }

    // enable: keeps the entries and reaches the manager now; when the manager is not there yet, the first tick tries
    // again. A lock poisoned by a panic before the last disable is cleared
    pub fn start(&self, entries: GameEntries) {
        if let Ok(calls) = MeshCalls::new(&entries) {
            let _ = self.mesh_calls.set(calls);
        }
        let _ = self.entries.set(entries);
        if self.state.is_poisoned() {
            self.state.clear_poison();
        }
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.link = LinkState::Waiting;
        state.epoch += 1;
        self.latch.lower();
        self.attach(&mut state, false);
    }

    // keeps an installed hook; false, keeping nothing, when HOOK_CAPACITY hooks are already kept
    pub fn keep_hook(&self, hook: HookHandle) -> bool {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.hooks.len() >= Self::HOOK_CAPACITY {
            return false;
        }
        state.hooks.push(hook);
        true
    }

    // the installed hooks, newest first, leaving none behind
    pub fn take_hooks(&self) -> Vec<HookHandle> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let mut hooks = std::mem::take(&mut state.hooks);
        hooks.reverse();
        hooks
    }

    // the body of the client update detour
    #[allow(unsafe_code)]
    pub fn tick(&self, client: NonNull<c_void>) {
        self.guarded("the tick", || {
            self.client.set(client.as_ptr());
            let Some(entries) = self.entries.get() else { return };
            let Ok(mut state) = self.state.lock() else { return };
            if state.link == LinkState::Waiting {
                self.attach(&mut state, true);
            }
            let RuntimeState { overlay, scan, sample_us, epoch, in_world, .. } = &mut *state;
            let Some(overlay) = overlay.as_mut() else { return };
            // SAFETY: `client` is the `this` of the running `ClientInstance::update`, live for this call
            let Some(client) = (unsafe { ClientInstance::from_raw(client.as_ptr(), entries) }) else { return };
            *sample_us = scan.tick(&client, overlay);
            if *in_world && !overlay.is_in_world() {
                *epoch += 1;
            }
            *in_world = overlay.is_in_world();
        });
    }

    // the body of the render-level hook: runs after the game drew the level, so the world meshes are depth tested
    // against it. `screen` is the ScreenContext and `frame` the FrameRenderObject of the call
    #[allow(unsafe_code)]
    pub fn render_world(&self, renderer: usize, screen: usize, frame: usize) {
        self.guarded("the render hook", || {
            let (Some(entries), Some(calls)) = (self.entries.get(), self.mesh_calls.get()) else {
                return;
            };
            // the tick may hold the lock; this frame is then skipped
            let Ok(mut state) = self.state.try_lock() else { return };
            let start = FrameStatusReader::now_seconds();
            let RuntimeState {
                overlay, world, material, name_tag, last_vibrant, epoch, world_epoch, sample_us, ..
            } = &mut *state;
            if *world_epoch != *epoch {
                world.leave();
                // SAFETY: `renderer` is the `this` of the running `renderLevel` call, a live LevelRenderer
                let current = unsafe { WireframeMaterial::from_level_renderer(renderer as *const c_void, entries) };
                // SAFETY: `current` was just read from the live renderer in this call
                unsafe { material.restore_if_current(current.as_ref(), entries) };
                *world_epoch = *epoch;
            }
            let Some(overlay) = overlay.as_mut() else {
                return;
            };
            if !overlay.is_in_world() {
                world.leave();
                // SAFETY: `renderer` is the `this` of the running `renderLevel` call, a live LevelRenderer
                let current = unsafe { WireframeMaterial::from_level_renderer(renderer as *const c_void, entries) };
                // SAFETY: `current` was just read from the live renderer in this call
                unsafe { material.restore_if_current(current.as_ref(), entries) };
                return;
            }
            let client = self.client.get(&self.latch);
            // SAFETY: the client is the last tick's, live while the world is, and a world is active in this call
            let vibrant = client.and_then(|client| unsafe { GraphicsMode::is_vibrant(client.as_ptr(), entries) });
            let vibrant = vibrant.unwrap_or(false);
            if *last_vibrant != Some(vibrant) {
                curia::info!("graphics mode seen, world material chosen", { vibrant: vibrant });
                *last_vibrant = Some(vibrant);
            }
            overlay.set_vibrant(vibrant);
            let Some(plan) = overlay.world_plan() else {
                return;
            };
            // SAFETY: `frame` is the FrameRenderObject of the running `renderLevel` call
            let Some(camera) = (unsafe { FrameRenderObject::camera(frame as *const c_void, calls, entries) }) else {
                return;
            };
            // SAFETY: `renderer` is the `this` of the running `renderLevel` call, a live LevelRenderer
            let wireframe = unsafe { WireframeMaterial::from_level_renderer(renderer as *const c_void, entries) };
            let draw_material = if vibrant {
                let Some(wireframe) = wireframe else {
                    return;
                };
                // SAFETY: the material was just read from the live renderer; this is the render thread
                unsafe { material.apply(&wireframe, entries) };
                wireframe.material_ptr()
            } else {
                // SAFETY: `wireframe` was just read from the live renderer in this call
                unsafe { material.restore_if_current(wireframe.as_ref(), entries) };
                // SAFETY: the allocator is the game heap's; this is the render thread inside the render call
                let Some(tag) = (unsafe { name_tag.material(entries, Self::allocate) }) else { return };
                tag
            };
            // the name-tag material culls back faces; the wireframe material draws both sides
            world.set_two_sided(!vibrant);
            let world_frame = WorldFrame {
                calls,
                screen: screen as *mut c_void,
                client: client.map_or(std::ptr::null_mut(), NonNull::as_ptr),
                material: draw_material,
                camera,
            };
            // SAFETY: the screen and frame are the live arguments of the call, the client the last tick's, the
            // material the level renderer's wireframe material (Vibrant Visuals) or the never-freed name-tag
            // material; this is the render thread
            let stats = unsafe { world.render(&world_frame, Some(plan)) };
            let total_us = ((FrameStatusReader::now_seconds() - start) * 1_000_000.0) as f32;
            overlay.timing(*sample_us, total_us, world.resident_bytes());
            overlay.world_timing(stats.build_us, stats.draw_us, stats.meshes_built);
        });
    }

    // disable, after the hooks are gone: the registration ends and the mod forgets the client
    pub fn stop(&self) {
        self.latch.raise();
        self.client.clear();
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.overlay = None;
        state.link = LinkState::Off;
    }

    pub fn is_attached(&self) -> bool {
        self.state.lock().map(|state| state.link == LinkState::Attached).unwrap_or(false)
    }

    // runs `body` unless the mod is stopped; a panic is logged once and raises the latch, so the hooks stop and the
    // game goes on
    fn guarded(&self, what: &str, body: impl FnOnce()) {
        if self.latch.raised() || ModRuntime::is_disabled() {
            return;
        }
        if let Err(payload) = catch_unwind(AssertUnwindSafe(body)) {
            self.latch.raise();
            if let Some(runtime) = ModRuntime::get() {
                runtime
                    .logger()
                    .error(&format!("{what} panicked: {}; the overlay stops", PanicText::of(payload.as_ref())));
            }
        }
    }

    fn attach(&self, state: &mut RuntimeState, last_try: bool) {
        match ApiTable::resolve().and_then(|table| Overlay::connect(table, &self.config_dir)) {
            Ok(overlay) => {
                state.overlay = Some(overlay);
                state.link = LinkState::Attached;
            }
            Err(error) if last_try => {
                Self::log_link_error(&error);
                state.link = LinkState::Off;
            }
            Err(_) => {}
        }
    }

    fn log_link_error(error: &LinkError) {
        let Some(runtime) = ModRuntime::get() else { return };
        match error {
            LinkError::ManagerMissing | LinkError::AbiMismatch { .. } => {
                runtime.logger().error("@alaydriem/levels needs @amml/modmanager; the overlay stays off");
            }
            other => {
                runtime.logger().error(&format!("@alaydriem/levels could not start ({other}); the overlay stays off"));
            }
        }
    }
}
