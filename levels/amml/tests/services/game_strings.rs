use alaydriem_levels::services::GameStrings;
use alaydriem_levels::types::GameString;

use crate::support::{FakeHeap, OwnImage};

unsafe extern "C" fn no_memory(_: usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[test]
fn no_text_lies_in_the_mods_own_image_and_nothing_is_freed() {
    let image = OwnImage::range();
    let frees_before = FakeHeap::frees();
    {
        let mut strings = GameStrings::new(FakeHeap::allocate);
        for text in GameString::ALL {
            let copy = strings.get(text).unwrap();

            assert_eq!(copy, text.text());
            assert!(!image.contains(&(copy.as_ptr() as usize)), "{text:?} is inside the image");
            assert!(image.contains(&(text.text().as_ptr() as usize)), "the source text is the image's");
        }
    }

    assert_eq!(FakeHeap::frees(), frees_before);
}

#[test]
fn each_text_is_copied_once() {
    let mut strings = GameStrings::new(FakeHeap::allocate);

    let first = strings.get(GameString::BorderMesh).unwrap();
    let again = strings.get(GameString::BorderMesh).unwrap();

    assert_eq!(first.as_ptr(), again.as_ptr());
    assert_ne!(first.as_ptr(), strings.get(GameString::ShellMesh).unwrap().as_ptr());
}

#[test]
fn a_heap_that_returns_null_gives_no_copy_and_tries_again_later() {
    let mut empty = GameStrings::new(no_memory);

    assert!(empty.get(GameString::WhitePath).is_none());
    assert!(empty.get(GameString::WhitePath).is_none());
}
