use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::{Mutex, OnceLock, PoisonError};

use amm_client::link::ApiTable;
use amm_core::errors::LinkError;
use amml_game::{ClientInstance, GameEntries, GameHeap, HeldClient, RenderCamera, StopLatch};
use amml_sdk::{HookHandle, ModRuntime, PanicText};

use crate::game::{BaseActorRenderer, FrameRenderObject};
use crate::services::{
    CameraStore, FrameStatusReader, GameMeshFactory, LabelDrawer, MeshFactory, Overlay, ScanDriver, WorldDrawer,
};
use crate::types::{DrawOutcome, DrawSetup, LevelsFeature, LinkState, MeshCalls, WorldFrame};

// what the detours share: the folders, the stop latch, the camera and the state of the current enable. The tick, the
// name-tag hook and the render-level hook may run on more than one thread; each takes the state lock, the two draw
// hooks with `try_lock`, so a draw frame never waits. Disable may run on another thread and waits for the lock
pub struct LevelsRuntime {
    config_dir: PathBuf,
    latch: StopLatch,
    entries: OnceLock<GameEntries>,
    // the world mesh path; set at the first enable when the markers feature is on
    mesh_calls: OnceLock<MeshCalls>,
    client: HeldClient,
    camera: CameraStore,
    state: Mutex<RuntimeState>,
}

// everything behind the lock. The drawer outlives a disable: the game may still hold a name-tag list in the ring.
// The world drawer's meshes and texture belong to the render device, so a disable does not free them: `epoch` moves
// on each enable and on each world leave the tick sees, and the next render frame whose `world_epoch` is behind it
// drops them first, on the render thread
pub struct RuntimeState {
    overlay: Option<Overlay>,
    link: LinkState,
    scan: ScanDriver,
    drawer: LabelDrawer,
    world: WorldDrawer,
    epoch: u64,
    world_epoch: u64,
    // whether the last tick saw a world loaded
    in_world: bool,
    hooks: Vec<HookHandle>,
    sample_us: f32,
}

// SAFETY: the raw pointers in the state are blocks of the game heap and addresses of game objects. They are used by
// the tick, the name-tag hook and the render-level hook, only under the state lock, and nowhere else
#[allow(unsafe_code)]
unsafe impl Send for RuntimeState {}

static RUNTIME: OnceLock<LevelsRuntime> = OnceLock::new();

impl LevelsRuntime {
    // the detours: client update, setup camera, view object, extract name tags, render level
    pub const HOOK_CAPACITY: usize = 5;

    pub fn new(config_dir: &Path) -> Self {
        Self {
            config_dir: config_dir.to_path_buf(),
            latch: StopLatch::new(),
            entries: OnceLock::new(),
            mesh_calls: OnceLock::new(),
            client: HeldClient::new(),
            camera: CameraStore::new(),
            state: Mutex::new(RuntimeState {
                overlay: None,
                link: LinkState::Off,
                scan: ScanDriver::new(),
                drawer: LabelDrawer::new(Self::allocate),
                world: WorldDrawer::new(MeshFactory::Game(GameMeshFactory::new(Self::allocate))),
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

    pub fn camera(&self) -> &CameraStore {
        &self.camera
    }

    pub fn entries(&self) -> Option<&GameEntries> {
        self.entries.get()
    }

    // the game heap's allocator, as a plain function for the ring and the renderer
    #[allow(unsafe_code)]
    unsafe extern "C" fn allocate(size: usize) -> *mut u8 {
        ModRuntime::get().map_or(std::ptr::null_mut(), |runtime| runtime.api().heap_new(size))
    }

    // enable: keeps the entries and reaches the manager now; when the manager is not there yet, the first tick tries
    // again. A lock poisoned by a panic before the last disable is cleared
    pub fn start(&self, entries: GameEntries) {
        if LevelsFeature::is_on(&entries, LevelsFeature::MARKERS)
            && let Ok(calls) = MeshCalls::new(&entries)
        {
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

    // the body of the name-tag hook: runs after the game made its tags. `tags` is the collection the game returned
    // and `screen` the ScreenContext of the call
    #[allow(unsafe_code)]
    pub fn draw(&self, tags: usize, screen: usize) -> DrawOutcome {
        let mut outcome = DrawOutcome::Skipped;
        self.guarded("the name-tag hook", || {
            let Some(entries) = self.entries.get() else { return };
            // the tick or the render-level hook may hold the lock; this frame is then skipped
            let Ok(mut state) = self.state.try_lock() else { return };
            let Some(heap) = GameHeap::msvc() else { return };
            let start = FrameStatusReader::now_seconds();
            let setup = DrawSetup {
                entries,
                heap: &heap,
                allocate: Self::allocate,
                client: self.client.get(&self.latch),
                screen: screen as *mut c_void,
            };
            let RuntimeState { overlay, drawer, world, sample_us, .. } = &mut *state;
            let Some(overlay) = overlay.as_mut() else { return };
            // SAFETY: `tags` is the collection the game just returned from `extractNameTags` and `screen` the
            // ScreenContext of the same call; the heap is the game's
            outcome = unsafe { drawer.append(&setup, overlay, self.camera.read(), tags as *mut c_void) };
            let draw_us = ((FrameStatusReader::now_seconds() - start) * 1_000_000.0) as f32;
            overlay.timing(*sample_us, draw_us, drawer.resident_bytes().saturating_add(world.resident_bytes()));
        });
        outcome
    }

    // the body of the render-level hook: runs after the game drew the level, so the world meshes are depth tested
    // against it. `screen` is the ScreenContext and `frame` the FrameRenderObject of the call
    #[allow(unsafe_code)]
    pub fn render_world(&self, _renderer: usize, screen: usize, frame: usize) {
        self.guarded("the render hook", || {
            let (Some(entries), Some(calls)) = (self.entries.get(), self.mesh_calls.get()) else { return };
            // the tick or the name-tag hook may hold the lock; this frame is then skipped
            let Ok(mut state) = self.state.try_lock() else { return };
            let RuntimeState { overlay, drawer, world, epoch, world_epoch, .. } = &mut *state;
            if *world_epoch != *epoch {
                world.leave();
                *world_epoch = *epoch;
            }
            let Some(overlay) = overlay.as_mut() else { return };
            if !overlay.is_in_world() {
                world.leave();
                return;
            }
            let Some(plan) = overlay.world_plan() else { return };
            // SAFETY: `frame` is the FrameRenderObject of the running `renderLevel` call
            let Some(camera) = (unsafe { FrameRenderObject::camera(frame as *const c_void, calls, entries) }) else {
                return;
            };
            let Some(heap) = GameHeap::msvc() else { return };
            let client = self.client.get(&self.latch);
            let setup =
                DrawSetup { entries, heap: &heap, allocate: Self::allocate, client, screen: screen as *mut c_void };
            // SAFETY: the heap functions are the game's
            let Some(address) = (unsafe { drawer.renderer(&setup) }) else { return };
            // SAFETY: the address came from `create` for this game build
            let Some(renderer) = (unsafe { BaseActorRenderer::from_address(address) }) else { return };
            let world_frame = WorldFrame {
                calls,
                screen: screen as *mut c_void,
                client: client.map_or(std::ptr::null_mut(), NonNull::as_ptr),
                material: renderer.tag_material(),
                camera,
            };
            // SAFETY: the screen and frame are the live arguments of the call, the client the last tick's, the
            // material the never-freed renderer's; this is the render thread
            let stats = unsafe { world.render(&world_frame, Some(plan)) };
            overlay.world_timing(stats.build_us, stats.draw_us, stats.meshes_built);
        });
    }

    // after `setupCamera`: keeps the direction the render camera faces
    #[allow(unsafe_code)]
    pub fn observe_camera(&self, camera: usize) {
        self.guarded("the camera hook", || {
            let Some(entries) = self.entries.get() else { return };
            // SAFETY: `camera` is the Camera argument of the `setupCamera` call that just returned, live for it
            if let Some(forward) = unsafe { RenderCamera::new(entries).forward(camera as *const c_void) } {
                self.camera.observe_forward(forward);
            }
        });
    }

    // after `createViewRenderObject`: keeps the camera position of the view object the game just filled
    #[allow(unsafe_code)]
    pub fn observe_view(&self, view_object: usize) {
        self.guarded("the view hook", || {
            let Some(entries) = self.entries.get() else { return };
            // SAFETY: `view_object` is the ViewRenderObject buffer that `createViewRenderObject` just filled
            if let Some(position) = unsafe { RenderCamera::new(entries).view_position(view_object as *const c_void) } {
                self.camera.observe_position(position);
            }
        });
    }

    // disable, after the hooks are gone: the registration ends and the mod forgets the client and the camera. The
    // drawer, with its ring, stays: the game may still hold a list in a ring slot
    pub fn stop(&self) {
        self.latch.raise();
        self.client.clear();
        self.camera.reset();
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
