use std::ffi::c_void;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Mutex;

use alaydriem_levels::services::{LabelDrawer, Overlay};
use alaydriem_levels::types::{DrawOutcome, DrawSetup, LevelsEntry};
use amm_client::test_support::FakeTable;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amml_game::GameEntries;
use amml_msvc::{MsvcString, MsvcVec, Optional};
use levels_core::structs::frame::FrameStatus;
use levels_core::structs::light::SurfaceSample;
use levels_core::structs::world::{BlockPos, ColumnPos};

use crate::support::{FakeHeap, FakeResolver, LogCapture};

const ELEMENT: usize = 144;
const TESSELLATOR_AT: u64 = 8;
const RENDERER_SIZE: u64 = 0xB8;
// the depth-tested materials in the renderer mirror, at the offsets both game builds derive
const TAG_MATERIAL_AT: u64 = 0x78;
const TEXT_MATERIAL_AT: u64 = 0x98;
const TESSELLATOR: usize = 0xDEAD_BEEF;

// what the fake `_extractRenderTextObject` saw
#[derive(Debug, Clone, PartialEq)]
struct ExtractCall {
    result: usize,
    tessellator: usize,
    text: String,
    widths: Vec<i32>,
    position: [f32; 3],
    text_color: [f32; 4],
    tag_material: usize,
    text_material: usize,
    tag_color: [f32; 4],
    scale: f32,
    rotation: Option<[f32; 4]>,
    line_gap: f32,
}

static CALLS: Mutex<Vec<ExtractCall>> = Mutex::new(Vec::new());
static RENDERERS: Mutex<Vec<usize>> = Mutex::new(Vec::new());

unsafe extern "C" fn fake_ctor(this: *mut u8) -> *mut u8 {
    RENDERERS.lock().unwrap().push(this as usize);
    // SAFETY: the mod passes a zeroed block of the class size
    unsafe { this.write(0x5A) };
    this
}

#[allow(clippy::too_many_arguments)]
unsafe extern "C" fn fake_extract(
    result: *mut u8,
    tessellator: *mut c_void,
    text: *const MsvcString,
    widths: *const MsvcVec<i32>,
    position: *const [f32; 3],
    text_color: *const [f32; 4],
    tag_material: *const c_void,
    text_material: *const c_void,
    tag_color: *const [f32; 4],
    scale: f32,
    rotation: *const Optional<[f32; 4]>,
    line_gap: f32,
) -> *mut u8 {
    // SAFETY: the mod passes live arguments of these types
    let call = unsafe {
        ExtractCall {
            result: result as usize,
            tessellator: tessellator as usize,
            text: (*text).to_str().unwrap().to_string(),
            widths: (*widths).as_slice().to_vec(),
            position: *position,
            text_color: *text_color,
            tag_material: tag_material as usize,
            text_material: text_material as usize,
            tag_color: *tag_color,
            scale,
            rotation: (*rotation).get().copied(),
            line_gap,
        }
    };
    let mut calls = CALLS.lock().unwrap();
    // SAFETY: the mod passes room for one object
    unsafe { result.write(0xE0 + u8::try_from(calls.len()).unwrap()) };
    calls.push(call);
    result
}

fn entries(size: u64) -> GameEntries {
    FakeResolver::entries(&[
        (LevelsEntry::NAME_TAG_SIZE, size),
        (LevelsEntry::SCREEN_CONTEXT_TESSELLATOR, TESSELLATOR_AT),
        (LevelsEntry::BASE_ACTOR_RENDERER_CTOR, fake_ctor as unsafe extern "C" fn(*mut u8) -> *mut u8 as usize as u64),
        (LevelsEntry::BASE_ACTOR_RENDERER_SIZE, RENDERER_SIZE),
        (
            LevelsEntry::EXTRACT_RENDER_TEXT_OBJECT,
            fake_extract
                as unsafe extern "C" fn(
                    *mut u8,
                    *mut c_void,
                    *const MsvcString,
                    *const MsvcVec<i32>,
                    *const [f32; 3],
                    *const [f32; 4],
                    *const c_void,
                    *const c_void,
                    *const [f32; 4],
                    f32,
                    *const Optional<[f32; 4]>,
                    f32,
                ) -> *mut u8 as usize as u64,
        ),
    ])
}

fn in_world(time_seconds: f64) -> FrameStatus {
    FrameStatus {
        time_seconds,
        in_world: true,
        has_player: true,
        menu_open: false,
        hud_hidden: false,
        dimension: 0,
        feet: BlockPos { x: 10, y: 64, z: -20 },
        sky_darken: 0,
        yaw: 90.0,
    }
}

// an overlay that is on, with one surface under the feet whose block light is 7
fn overlay_on() -> Overlay {
    FakeTable::set_config("");
    let mut overlay = Overlay::connect(FakeTable::table(), Path::new(FakeTable::CONFIG_DIR)).unwrap();
    overlay.frame(in_world(0.0));
    FakeTable::push_event(ModEvent::Keybind { action: 0, phase: KeybindPhase::Pressed });
    overlay.frame(FrameStatus { has_player: false, ..in_world(0.0) });
    overlay.frame(in_world(1.0 / 60.0));
    let sample = SurfaceSample { pos: BlockPos { x: 10, y: 63, z: -20 }, block_light: 7, sky_light: 0 };
    overlay.submit_column(ColumnPos { x: 10, z: -20 }, &[sample]);
    overlay
}

// a collection of 8 words with the vector pointers at word 2 to 4 and `tags` game tags in `storage`
fn collection(words: &mut [usize; 8], storage: &[u8], tags: usize) {
    let start = storage.as_ptr() as usize;
    words[2] = if tags == 0 { 0 } else { start };
    words[3] = if tags == 0 { 0 } else { start + tags * ELEMENT };
    words[4] = words[3];
}

fn setup<'a>(entries: &'a GameEntries, heap: &'a amml_msvc::MsvcHeap, screen: &'a mut [usize; 4]) -> DrawSetup<'a> {
    screen[1] = TESSELLATOR;
    DrawSetup {
        entries,
        heap,
        allocate: FakeHeap::allocate,
        client: Some(NonNull::dangling()),
        screen: (&raw mut *screen).cast::<c_void>(),
    }
}

#[test]
fn with_no_labels_the_games_list_comes_back_unchanged() {
    FakeTable::set_config("");
    let mut overlay = Overlay::connect(FakeTable::table(), Path::new(FakeTable::CONFIG_DIR)).unwrap();
    overlay.frame(in_world(0.0));
    let storage = vec![0x11_u8; 3 * ELEMENT];
    let mut words = [0; 8];
    collection(&mut words, &storage, 3);
    let before = words;
    let entries = entries(ELEMENT as u64);
    let heap = FakeHeap::msvc();
    let mut screen = [0; 4];
    let mut drawer = LabelDrawer::new(FakeHeap::allocate);

    // SAFETY: the fake collection and screen are live, and the heap functions are valid
    let outcome = unsafe {
        drawer.append(&setup(&entries, &heap, &mut screen), &mut overlay, None, (&raw mut words).cast::<c_void>())
    };

    assert_eq!(outcome, DrawOutcome::NoLabels);
    assert_eq!(words, before);
    assert!(!drawer.is_broken());
}

#[test]
fn a_layout_this_build_does_not_expect_stops_the_drawing_without_a_write() {
    for (size, pointers) in [(0, [0x1000, 0x1090, 0x1090]), (144, [0x2000, 0x1000, 0x3000])] {
        let mut overlay = overlay_on();
        let mut words = [0, 0, pointers[0], pointers[1], pointers[2], 0, 0, 0];
        let before = words;
        let entries = entries(size);
        let heap = FakeHeap::msvc();
        let mut screen = [0; 4];
        let mut drawer = LabelDrawer::new(FakeHeap::allocate);
        let tags = (&raw mut words).cast::<c_void>();

        // SAFETY: the fake collection and screen are live, and the heap functions are valid
        let first = unsafe { drawer.append(&setup(&entries, &heap, &mut screen), &mut overlay, None, tags) };
        // SAFETY: as above
        let second = unsafe { drawer.append(&setup(&entries, &heap, &mut screen), &mut overlay, None, tags) };

        assert_eq!((first, second), (DrawOutcome::Skipped, DrawOutcome::Skipped));
        assert!(drawer.is_broken());
        assert_eq!(words, before, "size {size}");
    }
    assert!(CALLS.lock().unwrap().is_empty());
}

#[test]
fn a_layout_fault_writes_one_log_line_however_often_the_hook_runs() {
    LogCapture::install();
    let mut overlay = overlay_on();
    let mut words = [0, 0, 0x2000, 0x1000, 0x3000, 0, 0, 0];
    let entries = entries(ELEMENT as u64);
    let heap = FakeHeap::msvc();
    let mut screen = [0; 4];
    let mut drawer = LabelDrawer::new(FakeHeap::allocate);
    let tags = (&raw mut words).cast::<c_void>();

    for _ in 0..3 {
        // SAFETY: the fake collection and screen are live, and the heap functions are valid
        unsafe { drawer.append(&setup(&entries, &heap, &mut screen), &mut overlay, None, tags) };
    }

    assert!(drawer.is_broken());
    assert_eq!(
        LogCapture::count(
            "@alaydriem/levels stops drawing: the name-tag list layout is not the one this build expects"
        ),
        1
    );
}

#[test]
fn the_labels_follow_the_games_tags_in_a_new_list_built_by_the_game_function() {
    let mut overlay = overlay_on();
    let storage: Vec<u8> = (0..2 * ELEMENT).map(|index| (index % 251) as u8).collect();
    let mut words = [0; 8];
    collection(&mut words, &storage, 2);
    let entries = entries(ELEMENT as u64);
    let heap = FakeHeap::msvc();
    let mut screen = [0; 4];
    let mut drawer = LabelDrawer::new(FakeHeap::allocate);

    // SAFETY: the fake collection and screen are live, and the heap functions are valid
    let outcome = unsafe {
        drawer.append(&setup(&entries, &heap, &mut screen), &mut overlay, None, (&raw mut words).cast::<c_void>())
    };

    assert_eq!(outcome, DrawOutcome::Drew(1));
    let [first, last, end] = [words[2], words[3], words[4]];
    assert_ne!(first, storage.as_ptr() as usize);
    assert_eq!(last - first, 3 * ELEMENT);
    assert_eq!(end, last);
    // SAFETY: the new list is in the drawer ring, which holds 3 elements
    let moved = unsafe { std::slice::from_raw_parts(first as *const u8, 3 * ELEMENT) };
    assert_eq!(moved[..2 * ELEMENT], storage[..]);
    assert_eq!(moved[2 * ELEMENT], 0xE0);

    let calls = CALLS.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let call = &calls[0];
    let renderer = RENDERERS.lock().unwrap()[0];
    assert_eq!(call.result, first + 2 * ELEMENT);
    assert_eq!(call.tessellator, TESSELLATOR);
    assert_eq!(call.text, "7");
    assert_eq!(call.widths, [alaydriem_levels::services::LabelText::fixed_width("7")]);
    assert_eq!(call.position[0], 10.5);
    assert_eq!(call.tag_material, renderer + TAG_MATERIAL_AT as usize);
    assert_eq!(call.text_material, renderer + TEXT_MATERIAL_AT as usize);
    assert_eq!(call.tag_color, [0.0; 4]);
    assert_eq!(call.line_gap, 0.0);
    assert!(call.rotation.is_some());
    assert!(call.scale > 0.0);
}
