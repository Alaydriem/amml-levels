use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;

use alaydriem_levels::LevelsMod;
use alaydriem_levels::types::LevelsEntry;
use amml_game::{CameraLayout, EntryId, GameError};
use amml_sdk::amml_abi::{AMML_API_VERSION, AmmlApi, AmmlLogLevel, AmmlStatus, AmmlStr};
use amml_sdk::{AmmlMod, Api};

use crate::support::FakeResolver;

// the size entries hold the sizes of the mirrors, as both game builds derive them
fn value(id: EntryId, other: u64) -> u64 {
    match id {
        LevelsEntry::MESH_SIZE => 0x260,
        LevelsEntry::BASE_ACTOR_RENDERER_SIZE => 0xB8,
        LevelsEntry::TESSELLATOR_TESSELLATING => 0x2A0,
        LevelsEntry::RESOURCE_LOCATION_FULL_HASH => 0x30,
        LevelsEntry::IMAGE_BUFFER_DESCRIPTION => 0x18,
        LevelsEntry::IMAGE_DESCRIPTION_DEPTH => 0x10,
        LevelsEntry::TEXTURE_GROUP_BASE_OFFSET => 0x18,
        LevelsEntry::SCREEN_CONTEXT_MESH_CONTEXT => 0x10,
        LevelsEntry::SCREEN_CONTEXT_CAMERA => 0x18,
        LevelsEntry::SCREEN_CONTEXT_CONSTANT_BUFFERS => 0x20,
        LevelsEntry::SCREEN_CONTEXT_CURRENT_SHADER_COLOR => 0x30,
        EntryId::CAMERA_FORWARD => CameraLayout::FORWARD,
        _ => other,
    }
}

// every required entry with a value
fn required() -> Vec<(EntryId, u64)> {
    LevelsMod::REQUIRED.iter().enumerate().map(|(index, id)| (*id, value(*id, 0x100 + index as u64))).collect()
}

fn with(values: Vec<(EntryId, u64)>, id: EntryId, value: u64) -> Vec<(EntryId, u64)> {
    values.into_iter().map(|(entry, old)| (entry, if entry == id { value } else { old })).collect()
}

#[test]
fn with_every_required_entry_the_wanted_list_is_met_and_no_layout_is_at_fault() {
    let entries = LevelsMod::resolve(&FakeResolver::new(&required())).unwrap();

    assert!(LevelsMod::layout_faults(&entries).is_empty());
}

#[test]
fn the_wanted_list_has_no_name_tag_or_font_entry() {
    let names: Vec<&str> = LevelsMod::wanted().iter().map(|(id, _)| id.as_str()).collect();

    for gone in
        ["extractNameTags", "NameTagRenderObject", "_extractRenderTextObject", "FontHandle", "Font.", "setupCamera"]
    {
        assert!(names.iter().all(|name| !name.contains(gone)), "{gone} is still wanted");
    }
    assert!(names.contains(&"LevelRenderer.renderLevel"));
    assert!(names.contains(&"LevelRendererCamera.mWireframeMaterial"));
}

// the world draw reads the camera position from the frame's first view through these two entries; without them
// every frame is skipped and nothing is drawn
#[test]
fn the_frame_camera_entries_are_wanted() {
    let names: Vec<&str> = LevelsMod::wanted().iter().map(|(id, _)| id.as_str()).collect();

    assert!(names.contains(&EntryId::VIEW_RENDER_OBJECT_VIEW_DATA.as_str()));
    assert!(names.contains(&EntryId::VIEW_RENDER_DATA_CAMERA_POS.as_str()));
}

#[test]
fn the_manifest_lists_every_required_entry() {
    let manifest = include_str!("../amml.hcl");

    for id in LevelsMod::REQUIRED {
        assert!(manifest.contains(&format!("\"{}\"", id.as_str())), "{} is not in amml.hcl", id.as_str());
    }
}

#[test]
fn a_mesh_size_that_fits_no_mirror_is_a_layout_fault() {
    let entries = LevelsMod::resolve(&FakeResolver::new(&with(required(), LevelsEntry::MESH_SIZE, 0x250))).unwrap();

    let faults = LevelsMod::layout_faults(&entries);

    assert_eq!(faults, ["Mesh.size is 0x250, the mirror expects 0x278"]);
}

#[test]
fn a_base_actor_renderer_size_that_fits_no_mirror_is_a_layout_fault() {
    let values = with(required(), LevelsEntry::BASE_ACTOR_RENDERER_SIZE, 0xC0);
    let entries = LevelsMod::resolve(&FakeResolver::new(&values)).unwrap();

    let faults = LevelsMod::layout_faults(&entries);

    assert_eq!(faults, ["BaseActorRenderer.size is 0xc0, the mirror is 0xb8"]);
}

#[test]
fn a_missing_mesh_entry_refuses_the_enable() {
    for missing in [LevelsEntry::RENDER_LEVEL, LevelsEntry::MESH_RENDER, LevelsEntry::WIREFRAME_MATERIAL] {
        let values: Vec<_> = required().into_iter().filter(|(id, _)| *id != missing).collect();

        let result = LevelsMod::resolve(&FakeResolver::new(&values));

        assert!(matches!(result, Err(GameError::MissingEntry { id }) if id == missing.as_str()), "{missing:?}");
    }
}

// a loader table whose functions are fakes: `resolve` answers from `VALUES`, `hook` and `unhook` record their calls
static VALUES: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);
static HOOKS: Mutex<Vec<(u64, i32)>> = Mutex::new(Vec::new());
static UNHOOKED: Mutex<Vec<u64>> = Mutex::new(Vec::new());
// a target whose hook the fake loader refuses
static REFUSED: Mutex<Option<u64>> = Mutex::new(None);
static LOGGED: Mutex<Vec<String>> = Mutex::new(Vec::new());
static DIR: &str = "C:/levels-test";

struct FakeLoader;

impl FakeLoader {
    unsafe extern "C" fn resolve(id: AmmlStr, out: *mut u64) -> AmmlStatus {
        // SAFETY: the SDK passes a live string view and a live u64
        let id = unsafe { id.as_str() }.unwrap().to_string();
        match VALUES.lock().unwrap().as_ref().and_then(|values| values.get(&id)) {
            Some(value) => {
                // SAFETY: as above
                unsafe { out.write(*value) };
                AmmlStatus::OK
            }
            None => AmmlStatus::MISSING_ENTRY,
        }
    }

    unsafe extern "C" fn hook(
        target: *const c_void,
        _detour: *const c_void,
        priority: i32,
        _original: *mut *const c_void,
        handle: *mut u64,
    ) -> AmmlStatus {
        if *REFUSED.lock().unwrap() == Some(target as u64) {
            return AmmlStatus::HOOK_FAILED;
        }
        let mut hooks = HOOKS.lock().unwrap();
        hooks.push((target as u64, priority));
        // SAFETY: the SDK passes a live u64
        unsafe { handle.write(hooks.len() as u64) };
        AmmlStatus::OK
    }

    unsafe extern "C" fn unhook(handle: u64) -> AmmlStatus {
        UNHOOKED.lock().unwrap().push(handle);
        AmmlStatus::OK
    }

    unsafe extern "C" fn log(_: AmmlStr, _: AmmlLogLevel, line: AmmlStr) {
        // SAFETY: the SDK passes a live string view
        let line = unsafe { line.as_str() }.unwrap().to_string();
        LOGGED.lock().unwrap().push(line);
    }

    unsafe extern "C" fn heap_new(_: usize) -> *mut u8 {
        std::ptr::null_mut()
    }

    unsafe extern "C" fn heap_delete(_: *mut u8, _: usize) {}

    unsafe extern "C" fn mod_dir(_: AmmlStr, _: u8, out: *mut AmmlStr) -> AmmlStatus {
        // SAFETY: the SDK passes a live view to write; the text is a static
        unsafe { out.write(AmmlStr::from_str(DIR)) };
        AmmlStatus::OK
    }

    fn api() -> Api {
        let table = AmmlApi {
            size: AmmlApi::SIZE_V1,
            version: AMML_API_VERSION,
            module_base: 0x1000,
            resolve: Self::resolve,
            hook: Self::hook,
            unhook: Self::unhook,
            log: Self::log,
            heap_new: Self::heap_new,
            heap_delete: Self::heap_delete,
            mod_dir: Self::mod_dir,
        };
        // SAFETY: the table is live for the call, and complete
        unsafe { Api::from_raw(&table, AMML_API_VERSION) }.unwrap()
    }
}

impl FakeLoader {
    // installs `entries` as the loader's values
    fn install(entries: Vec<(EntryId, u64)>) {
        let mut values: HashMap<String, u64> = HashMap::new();
        for (id, value) in entries {
            values.insert(id.as_str().to_string(), value);
        }
        *VALUES.lock().unwrap() = Some(values);
        HOOKS.lock().unwrap().clear();
        UNHOOKED.lock().unwrap().clear();
        *REFUSED.lock().unwrap() = None;
    }
}

fn address(id: EntryId) -> u64 {
    0x100 + LevelsMod::REQUIRED.iter().position(|entry| *entry == id).unwrap() as u64
}

#[test]
fn enable_hooks_the_update_and_the_level_render_after_the_writers() {
    FakeLoader::install(required());
    let api = FakeLoader::api();

    LevelsMod::load(&api).unwrap();
    LevelsMod::enable(&api).unwrap();

    let hooks = HOOKS.lock().unwrap().clone();
    assert_eq!(hooks, [(address(EntryId::CLIENT_INSTANCE_UPDATE), 100), (address(LevelsEntry::RENDER_LEVEL), 100)]);

    LevelsMod::disable(&api);

    assert_eq!(UNHOOKED.lock().unwrap().clone(), [2, 1]);
}

#[test]
fn a_layout_that_does_not_fit_refuses_the_enable_and_installs_no_hook() {
    FakeLoader::install(with(required(), LevelsEntry::MESH_SIZE, 0x250));
    let api = FakeLoader::api();

    LevelsMod::load(&api).unwrap();

    assert!(LevelsMod::enable(&api).is_err());
    assert!(HOOKS.lock().unwrap().is_empty());
}

#[test]
fn a_refused_render_hook_refuses_the_enable_and_removes_the_update_hook() {
    FakeLoader::install(required());
    *REFUSED.lock().unwrap() = Some(address(LevelsEntry::RENDER_LEVEL));
    let api = FakeLoader::api();

    LevelsMod::load(&api).unwrap();

    assert!(LevelsMod::enable(&api).is_err());
    assert_eq!(HOOKS.lock().unwrap().len(), 1);
    assert_eq!(UNHOOKED.lock().unwrap().clone(), [1]);
}

#[test]
fn the_mod_id_is_the_namespaced_module_string() {
    assert_eq!(LevelsMod::NAMESPACE, "alaydriem");
    assert_eq!(LevelsMod::NAME, "levels");
    assert_eq!(LevelsMod::id(), LevelsMod::MODULE);
    assert_eq!(LevelsMod::MODULE, "@alaydriem/levels");
}
