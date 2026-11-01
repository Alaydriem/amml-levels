use alaydriem_levels::services::LabelText;
use alaydriem_levels::types::WidthSource;

use crate::support::FakeHeap;

#[test]
fn without_a_font_the_fixed_widths_are_used() {
    let mut text = LabelText::new();

    // SAFETY: the heap functions are valid allocate and free functions
    let source = unsafe { text.prepare(&FakeHeap::msvc(), |_| None) };

    assert_eq!(source, Some(WidthSource::FixedTable));
    assert_eq!(text.source(), Some(WidthSource::FixedTable));
    assert_eq!(text.text(7).unwrap().to_str().unwrap(), "7");
    assert_eq!(text.widths(7).unwrap().as_slice(), [LabelText::fixed_width("7")]);
    assert_eq!(text.text(12).unwrap().to_str().unwrap(), "12");
    assert_eq!(text.widths(12).unwrap().as_slice(), [2 * LabelText::fixed_width("7")]);
}

#[test]
fn measured_widths_are_kept_in_the_order_of_the_texts() {
    let mut text = LabelText::new();
    let widths: Vec<i32> = (0..16).map(|value| 100 + value).collect();
    let expected = widths.clone();

    // SAFETY: as above
    let source = unsafe {
        text.prepare(&FakeHeap::msvc(), |texts| {
            assert_eq!(texts.len(), 16);
            assert_eq!(texts[0], "0");
            assert_eq!(texts[15], "15");
            Some(widths)
        })
    };

    assert_eq!(source, Some(WidthSource::Font));
    for value in 0..16u8 {
        assert_eq!(text.widths(value).unwrap().as_slice(), [expected[usize::from(value)]]);
    }
}

#[test]
fn a_wrong_count_of_widths_falls_back_to_the_fixed_table() {
    let mut text = LabelText::new();

    // SAFETY: as above
    let source = unsafe { text.prepare(&FakeHeap::msvc(), |_| Some(vec![1, 2, 3])) };

    assert_eq!(source, Some(WidthSource::FixedTable));
}

#[test]
fn texts_are_made_once() {
    let mut text = LabelText::new();
    let mut calls = 0;

    for _ in 0..3 {
        // SAFETY: as above
        unsafe {
            text.prepare(&FakeHeap::msvc(), |_| {
                calls += 1;
                None
            })
        };
    }

    assert_eq!(calls, 1);
}

#[test]
fn a_value_past_fifteen_shows_the_largest_text() {
    let mut text = LabelText::new();
    // SAFETY: as above
    unsafe { text.prepare(&FakeHeap::msvc(), |_| None) };

    assert_eq!(text.text(200).unwrap().to_str().unwrap(), "15");
}

#[test]
fn only_prepared_texts_cover_labels() {
    let mut text = LabelText::new();
    assert!(!text.covers([0]));
    assert!(text.covers([]));

    // SAFETY: as above
    unsafe { text.prepare(&FakeHeap::msvc(), |_| None) };

    assert!(text.covers(0..=u8::MAX));
}
