use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;

use alaydriem_levels::LevelsMod;
use alaydriem_levels::types::{LevelsEntry, LevelsFeature};
use amml_game::{CameraLayout, EntryId, GameError};
use amml_sdk::amml_abi::{AMML_API_VERSION, AmmlApi, AmmlLogLevel, AmmlStatus, AmmlStr};
use amml_sdk::{AmmlMod, Api};

use crate::support::{FakeResolver, FakeVtable};

// the size entries hold the sizes of the mirrors, as both game builds derive them
fn value(id: EntryId, other: u64) -> u64 {
    match id {
        LevelsEntry::BASE_ACTOR_RENDERER_SIZE => 0xB8,
        LevelsEntry::NAME_TAG_SIZE => 0x90,
        LevelsEntry::FONT_HANDLE_SIZE => 0x50,
        LevelsEntry::MESH_SIZE => 0x260,
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

// every required entry with a value; the optional entries are left out
fn required() -> Vec<(EntryId, u64)> {
    LevelsMod::REQUIRED.iter().enumerate().map(|(index, id)| (*id, value(*id, 0x100 + index as u64))).collect()
}

fn font() -> Vec<(EntryId, u64)> {
    LevelsFeature::entries(LevelsFeature::FONT).iter().map(|id| (*id, value(*id, 0x20))).collect()
}

fn markers() -> Vec<(EntryId, u64)> {
    LevelsFeature::entries(LevelsFeature::MARKERS).iter().map(|id| (*id, value(*id, 0x40))).collect()
}

fn with(values: Vec<(EntryId, u64)>, id: EntryId, value: u64) -> Vec<(EntryId, u64)> {
    values.into_iter().map(|(entry, old)| (entry, if entry == id { value } else { old })).collect()
}

#[test]
fn without_the_font_entries_the_font_feature_is_off_with_one_line_and_the_mod_still_resolves() {
    let values: Vec<_> = required().into_iter().chain(markers()).collect();

    let entries = LevelsMod::resolve(&FakeResolver::new(&values)).unwrap();

    let off: Vec<&str> = entries.off_features().into_iter().map(|(feature, _)| feature).collect();
    let lines = LevelsMod::off_lines(&entries);

    assert_eq!(off, [LevelsFeature::FONT]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("font is off"), "{lines:?}");
    assert!(lines[0].contains("fixed width table"), "{lines:?}");
    assert!(!LevelsFeature::is_on(&entries, LevelsFeature::FONT));
}

#[test]
fn with_every_entry_no_feature_is_off() {
    let values: Vec<_> = required().into_iter().chain(font()).chain(markers()).collect();

    let entries = LevelsMod::resolve(&FakeResolver::new(&values)).unwrap();

    assert!(entries.off_features().is_empty());
    assert!(LevelsMod::off_lines(&entries).is_empty());
    assert!(LevelsFeature::is_on(&entries, LevelsFeature::FONT));
    assert!(LevelsFeature::is_on(&entries, LevelsFeature::MARKERS));
}

#[test]
fn a_font_handle_size_that_is_not_the_mirror_size_turns_the_font_feature_off_with_one_line() {
    let values: Vec<_> =
        required().into_iter().chain(with(font(), LevelsEntry::FONT_HANDLE_SIZE, 0x58)).chain(markers()).collect();

    let entries = LevelsMod::resolve(&FakeResolver::new(&values)).unwrap();
    let lines = LevelsMod::off_lines(&entries);

    assert!(!LevelsFeature::is_on(&entries, LevelsFeature::FONT));
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("font is off (FontHandle.size is 0x58, the mirror is 0x50)"), "{lines:?}");
    assert!(LevelsMod::layout_faults(&entries).is_empty());
}

#[test]
fn a_mesh_size_that_fits_no_mirror_turns_markers_off_with_one_line_and_keeps_labels() {
    let values: Vec<_> =
        required().into_iter().chain(font()).chain(with(markers(), LevelsEntry::MESH_SIZE, 0x250)).collect();

    let entries = LevelsMod::resolve(&FakeResolver::new(&values)).unwrap();
    let lines = LevelsMod::off_lines(&entries);

    assert!(!LevelsFeature::is_on(&entries, LevelsFeature::MARKERS));
    assert_eq!(
        lines,
        [
            "markers is off (Mesh.size is 0x250, the mirror expects 0x278); chunk borders and spawn spheres are not drawn"
        ]
    );
    assert!(LevelsMod::layout_faults(&entries).is_empty());
}

#[test]
fn a_required_size_that_is_not_the_mirror_size_is_a_layout_fault() {
    for (id, size) in [(LevelsEntry::BASE_ACTOR_RENDERER_SIZE, 0xC0), (LevelsEntry::NAME_TAG_SIZE, 0x98)] {
        let entries = LevelsMod::resolve(&FakeResolver::new(&with(required(), id, size))).unwrap();

        let faults = LevelsMod::layout_faults(&entries);

        assert_eq!(faults.len(), 1, "{id:?}");
        assert!(faults[0].starts_with(id.as_str()), "{faults:?}");
    }
    let entries = LevelsMod::resolve(&FakeResolver::new(&required())).unwrap();
    assert!(LevelsMod::layout_faults(&entries).is_empty());
}

#[test]
fn a_missing_required_entry_refuses_the_enable() {
    let values: Vec<_> =
        required().into_iter().filter(|(id, _)| *id != LevelsEntry::EXTRACT_RENDER_TEXT_OBJECT).collect();

    let result = LevelsMod::resolve(&FakeResolver::new(&values));

    assert!(
        matches!(result, Err(GameError::MissingEntry { id }) if id == "BaseActorRenderer._extractRenderTextObject")
    );
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
    // installs `entries` as the loader's values, with a fake vtable whose extractNameTags slot holds a function;
    // gives that function's address
    fn install(entries: Vec<(EntryId, u64)>) -> usize {
        extern "C" fn dummy() {}
        let function = dummy as extern "C" fn() as usize;
        let vtable = FakeVtable::leak(&[0, function]);
        let mut values: HashMap<String, u64> = HashMap::new();
        for (id, value) in entries {
            values.insert(id.as_str().to_string(), value);
        }
        values.insert(LevelsEntry::LEVEL_RENDERER_PLAYER_VTABLE.as_str().to_string(), vtable as usize as u64);
        values.insert(LevelsEntry::EXTRACT_NAME_TAGS.as_str().to_string(), 2);
        *VALUES.lock().unwrap() = Some(values);
        function
    }
}

#[test]
fn enable_hooks_the_three_readers_after_the_writers_and_the_name_tag_list_as_a_writer() {
    let function = FakeLoader::install(required());
    let api = FakeLoader::api();

    LevelsMod::load(&api).unwrap();
    LevelsMod::enable(&api).unwrap();

    let hooks = HOOKS.lock().unwrap().clone();
    let update =
        0x100 + LevelsMod::REQUIRED.iter().position(|id| *id == EntryId::CLIENT_INSTANCE_UPDATE).unwrap() as u64;
    let setup = 0x100
        + LevelsMod::REQUIRED.iter().position(|id| *id == EntryId::LEVEL_RENDERER_PLAYER_SETUP_CAMERA).unwrap() as u64;
    let view = 0x100
        + LevelsMod::REQUIRED
            .iter()
            .position(|id| *id == EntryId::LEVEL_RENDERER_PLAYER_CREATE_VIEW_RENDER_OBJECT)
            .unwrap() as u64;
    assert_eq!(hooks, [(update, 100), (setup, 100), (view, 100), (function as u64, 0)]);

    LevelsMod::disable(&api);

    assert_eq!(UNHOOKED.lock().unwrap().clone(), [4, 3, 2, 1]);
}

#[test]
fn missing_render_entry_turns_markers_off_and_keeps_labels() {
    let values: Vec<_> = required()
        .into_iter()
        .chain(font())
        .chain(markers().into_iter().filter(|(id, _)| *id != LevelsEntry::RENDER_LEVEL))
        .collect();

    let entries = LevelsMod::resolve(&FakeResolver::new(&values)).unwrap();
    let lines = LevelsMod::off_lines(&entries);

    assert!(!LevelsFeature::is_on(&entries, LevelsFeature::MARKERS));
    assert!(LevelsFeature::is_on(&entries, LevelsFeature::FONT));
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("markers is off ("), "{lines:?}");
    assert!(lines[0].contains("LevelRenderer.renderLevel"), "{lines:?}");
    assert!(lines[0].ends_with("; chunk borders and spawn spheres are not drawn"), "{lines:?}");
    assert!(LevelsMod::layout_faults(&entries).is_empty());
    assert!(LevelsFeature::entries(LevelsFeature::MARKERS).iter().all(|id| !LevelsMod::REQUIRED.contains(id)));

    FakeLoader::install(values);
    let api = FakeLoader::api();
    LevelsMod::load(&api).unwrap();

    assert!(LevelsMod::enable(&api).is_ok());

    LevelsMod::disable(&api);
}

#[test]
fn markers_off_installs_four_hooks() {
    FakeLoader::install(required().into_iter().chain(font()).collect());
    let api = FakeLoader::api();

    LevelsMod::load(&api).unwrap();
    LevelsMod::enable(&api).unwrap();

    assert_eq!(HOOKS.lock().unwrap().len(), 4);

    LevelsMod::disable(&api);

    assert_eq!(UNHOOKED.lock().unwrap().clone(), [4, 3, 2, 1]);
}

#[test]
fn markers_on_installs_five_hooks() {
    let values: Vec<_> = required().into_iter().chain(font()).chain(markers()).collect();
    let render = values.iter().find(|(id, _)| *id == LevelsEntry::RENDER_LEVEL).unwrap().1;
    FakeLoader::install(values);
    let api = FakeLoader::api();

    LevelsMod::load(&api).unwrap();
    LevelsMod::enable(&api).unwrap();

    let hooks = HOOKS.lock().unwrap().clone();
    assert_eq!(hooks.len(), 5);
    assert_eq!(hooks[4], (render, 100));

    LevelsMod::disable(&api);

    assert_eq!(UNHOOKED.lock().unwrap().clone(), [5, 4, 3, 2, 1]);
}

#[test]
fn a_failed_render_hook_turns_markers_off_with_one_line_and_keeps_the_light_numbers() {
    let values: Vec<_> = required().into_iter().chain(font()).chain(markers()).collect();
    let render = values.iter().find(|(id, _)| *id == LevelsEntry::RENDER_LEVEL).unwrap().1;
    *REFUSED.lock().unwrap() = Some(render);
    FakeLoader::install(values);
    let api = FakeLoader::api();

    LevelsMod::load(&api).unwrap();
    assert!(LevelsMod::enable(&api).is_ok());

    assert_eq!(HOOKS.lock().unwrap().len(), 4);
    let off: Vec<String> =
        LOGGED.lock().unwrap().iter().filter(|line| line.starts_with("markers is off (")).cloned().collect();
    assert_eq!(off.len(), 1, "{off:?}");
    assert!(off[0].contains("render-level hook"), "{off:?}");

    LevelsMod::disable(&api);

    assert_eq!(UNHOOKED.lock().unwrap().clone(), [4, 3, 2, 1]);
}

#[test]
fn the_mod_id_is_the_namespaced_module_string() {
    assert_eq!(LevelsMod::NAMESPACE, "alaydriem");
    assert_eq!(LevelsMod::NAME, "levels");
    assert_eq!(LevelsMod::id(), LevelsMod::MODULE);
    assert_eq!(LevelsMod::MODULE, "@alaydriem/levels");
}
