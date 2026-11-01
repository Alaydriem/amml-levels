use std::ffi::c_void;
use std::sync::Mutex;

use alaydriem_levels::game::{FontReader, StringView};
use alaydriem_levels::types::LevelsEntry;
use amml_game::EntryId;

use crate::support::{FakeResolver, FakeVtable};

// what the fake game code saw, in call order
#[derive(Debug, PartialEq)]
enum Event {
    GetHandle { this: usize, buffer: usize },
    LineLength { font: usize, text: String, size: f32, flag: u8 },
    Destroy { this: usize },
}

static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());

// the fake font object; the handle the fake client fills points to it as the default font
static FONT_OBJECT: Mutex<usize> = Mutex::new(0);

extern "C" fn fake_get_font_handle(this: *mut c_void, buffer: *mut c_void) -> *mut c_void {
    EVENTS.lock().unwrap().push(Event::GetHandle { this: this as usize, buffer: buffer as usize });
    let font = *FONT_OBJECT.lock().unwrap() as u64;
    // SAFETY: the reader passes a FontHandle buffer of 80 bytes: the default font at 0x30, the id at 0x40
    unsafe {
        let words = buffer.cast::<u64>();
        words.add(6).write(font);
        words.add(8).write(u64::MAX);
    }
    buffer
}

extern "C" fn fake_get_line_length(font: *mut c_void, view: *const StringView, size: f32, flag: u8) -> i32 {
    // SAFETY: the reader passes a pointer to a string view that lives for the call
    let text = unsafe {
        let view = view.read();
        String::from_utf8_lossy(std::slice::from_raw_parts(view.data, view.len)).into_owned()
    };
    let width = text.len() as i32 * 7;
    EVENTS.lock().unwrap().push(Event::LineLength { font: font as usize, text, size, flag });
    width
}

extern "C" fn fake_destroy_font_handle(this: *mut c_void) {
    EVENTS.lock().unwrap().push(Event::Destroy { this: this as usize });
}

struct FakeGame {
    client: *mut c_void,
    font: usize,
    values: Vec<(EntryId, u64)>,
}

impl FakeGame {
    fn new() -> Self {
        EVENTS.lock().unwrap().clear();
        let font_vtable = FakeVtable::leak(&[fake_get_line_length as *const () as usize]);
        let font = Box::leak(Box::new([font_vtable as usize])).as_mut_ptr() as usize;
        *FONT_OBJECT.lock().unwrap() = font;
        let client_vtable = FakeVtable::leak(&[fake_get_font_handle as *const () as usize]);
        let client = Box::leak(Box::new([client_vtable as usize])).as_mut_ptr().cast::<c_void>();
        let values = vec![
            (LevelsEntry::GET_FONT_HANDLE, 1),
            (LevelsEntry::FONT_GET_LINE_LENGTH, 1),
            (LevelsEntry::FONT_HANDLE_DTOR, fake_destroy_font_handle as *const () as usize as u64),
            (LevelsEntry::FONT_HANDLE_SIZE, 80),
        ];
        Self { client, font, values }
    }

    fn without(mut self, id: EntryId) -> Self {
        self.values.retain(|(entry, _)| *entry != id);
        self
    }

    fn with(mut self, id: EntryId, value: u64) -> Self {
        self.values.retain(|(entry, _)| *entry != id);
        self.values.push((id, value));
        self
    }

    fn measure(&self, texts: &[&str]) -> Option<Vec<i32>> {
        let entries = FakeResolver::entries(&self.values);
        // SAFETY: the client is a fake object whose vtable holds the fake `getFontHandle`, and the entries name
        // the fake functions with the signatures the reader calls
        unsafe { FontReader::measure(&entries, self.client, texts) }
    }
}

#[test]
fn measure_passes_the_buffer_the_string_view_and_the_size_in_the_registers_the_game_reads() {
    let game = FakeGame::new();

    let widths = game.measure(&["ab", "hello"]);

    assert_eq!(widths, Some(vec![14, 35]));
    let events = EVENTS.lock().unwrap();
    let Event::GetHandle { this, buffer } = events[0] else { panic!("the first call is getFontHandle: {events:?}") };
    assert_eq!(this, game.client as usize);
    assert_eq!(
        events[1..3],
        [
            Event::LineLength { font: game.font, text: "ab".to_string(), size: 1.0, flag: 0 },
            Event::LineLength { font: game.font, text: "hello".to_string(), size: 1.0, flag: 0 },
        ]
    );
    assert_eq!(events[3], Event::Destroy { this: buffer });
    assert_eq!(events.len(), 4);
}

#[test]
fn measure_destroys_the_handle_once_when_no_font_is_found() {
    let game = FakeGame::new();
    *FONT_OBJECT.lock().unwrap() = 0;

    assert_eq!(game.measure(&["ab"]), None);

    let events = EVENTS.lock().unwrap();
    assert!(matches!(events[0], Event::GetHandle { .. }));
    assert!(matches!(events[1], Event::Destroy { .. }));
    assert_eq!(events.len(), 2);
}

#[test]
fn measure_calls_nothing_when_an_entry_is_missing() {
    for missing in [
        LevelsEntry::GET_FONT_HANDLE,
        LevelsEntry::FONT_HANDLE_DTOR,
        LevelsEntry::FONT_HANDLE_SIZE,
        LevelsEntry::FONT_GET_LINE_LENGTH,
    ] {
        let game = FakeGame::new().without(missing);

        assert_eq!(game.measure(&["ab"]), None, "{missing:?} missing");
        assert!(EVENTS.lock().unwrap().is_empty(), "{missing:?} missing");
    }
}

#[test]
fn measure_calls_nothing_when_the_font_handle_size_is_not_the_mirror_size() {
    for size in [0x48, 0x58] {
        let game = FakeGame::new().with(LevelsEntry::FONT_HANDLE_SIZE, size);

        assert_eq!(game.measure(&["ab"]), None, "size {size:#x}");
        assert!(EVENTS.lock().unwrap().is_empty(), "size {size:#x}");
    }
}
