use alaydriem_levels::services::LabelRing;

use crate::support::{FakeHeap, OwnImage};

#[test]
fn no_slot_lies_in_the_mods_own_image_and_nothing_is_freed() {
    let image = OwnImage::range();
    let frees_before = FakeHeap::frees();
    {
        let mut ring = LabelRing::new(FakeHeap::allocate);
        for round in 0..3 * LabelRing::SLOTS {
            let slot = ring.next(1_000 + round * 100).unwrap();
            assert!(!image.contains(&(slot as usize)), "slot {round} is inside the image");
        }
    }

    assert_eq!(FakeHeap::frees(), frees_before);
}

#[test]
fn a_slot_is_reused_a_ring_later_and_a_growing_slot_keeps_its_old_block() {
    let mut ring = LabelRing::new(FakeHeap::allocate);
    let first = ring.next(256).unwrap();
    for _ in 1..LabelRing::SLOTS {
        ring.next(256).unwrap();
    }

    let again = ring.next(128).unwrap();
    assert_eq!(again, first);

    for _ in 1..LabelRing::SLOTS {
        ring.next(128).unwrap();
    }
    let frees_before = FakeHeap::frees();
    let grown = ring.next(4096).unwrap();

    assert_ne!(grown, first);
    assert_eq!(FakeHeap::frees(), frees_before);
}

#[test]
fn a_heap_that_returns_null_gives_no_slot_and_keeps_the_ring_usable() {
    unsafe extern "C" fn refuse(_: usize) -> *mut u8 {
        std::ptr::null_mut()
    }
    let mut ring = LabelRing::new(refuse);

    assert!(ring.next(64).is_none());
    assert_eq!(ring.resident_bytes(), 0);
}

#[test]
fn resident_bytes_counts_every_slot() {
    let mut ring = LabelRing::new(FakeHeap::allocate);
    ring.next(100).unwrap();
    ring.next(200).unwrap();

    assert_eq!(ring.resident_bytes(), 112 + 208);
}
