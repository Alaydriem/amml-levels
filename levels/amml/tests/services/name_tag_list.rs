use std::ffi::c_void;

use alaydriem_levels::errors::LayoutFault;
use alaydriem_levels::game::NameTagCollection;
use alaydriem_levels::services::NameTagList;
use alaydriem_levels::types::LevelsEntry;

use crate::support::FakeResolver;

// a collection of 8 words: the allocator in the first two, the vector pointers in words 2 to 4
fn collection(words: &mut [usize; 8], size: u64) -> (NameTagCollection, amml_game::GameEntries) {
    let entries = FakeResolver::entries(&[(LevelsEntry::NAME_TAG_SIZE, size)]);
    // SAFETY: the array outlives the wrapper and holds the three pointers where the collection mirror has them
    let wrapper = unsafe { NameTagCollection::from_raw((&raw mut *words).cast::<c_void>(), &entries) }.unwrap();
    (wrapper, entries)
}

#[test]
fn a_list_of_whole_elements_opens_with_its_count() {
    let mut words = [0, 0, 0x1000, 0x1000 + 3 * 144, 0x1000 + 8 * 144, 0, 0, 0];
    let (collection, _entries) = collection(&mut words, 144);

    let list = NameTagList::open(collection).unwrap();

    assert_eq!(list.len(), 3);
    assert_eq!(list.element_size(), 144);
}

#[test]
fn an_empty_vector_with_null_pointers_opens_empty() {
    let mut words = [0; 8];
    let (collection, _entries) = collection(&mut words, 144);

    assert!(NameTagList::open(collection).unwrap().is_empty());
}

#[test]
fn an_element_size_that_is_not_the_mirror_size_is_refused_and_the_list_is_not_written() {
    let mut words = [0, 0, 0x1000, 0x1090, 0x1090, 0, 0, 0];
    let before = words;
    let (collection, _entries) = collection(&mut words, 0);

    assert_eq!(NameTagList::open(collection).err(), Some(LayoutFault::ElementSize));
    assert_eq!(words, before);
}

#[test]
fn pointers_out_of_order_are_refused() {
    for pointers in [[0x2000, 0x1000, 0x3000], [0x1000, 0x3000, 0x2000], [0, 0x1000, 0x2000]] {
        let mut words = [0, 0, pointers[0], pointers[1], pointers[2], 0, 0, 0];
        let (collection, _entries) = collection(&mut words, 144);

        assert_eq!(NameTagList::open(collection).err(), Some(LayoutFault::Unordered), "{pointers:?}");
    }
}

#[test]
fn a_length_that_is_not_a_whole_number_of_elements_is_refused() {
    let mut words = [0, 0, 0x1000, 0x1000 + 145, 0x2000, 0, 0, 0];
    let (collection, _entries) = collection(&mut words, 144);

    assert_eq!(NameTagList::open(collection).err(), Some(LayoutFault::Misaligned));
}

#[test]
fn move_to_copies_the_game_tags_and_repoints_first_last_and_end() {
    let game_storage = vec![0xAB_u8; 2 * 144];
    let start = game_storage.as_ptr() as usize;
    let mut words = [0, 0, start, start + 2 * 144, start + 2 * 144, 0, 0, 0];
    let (collection, _entries) = collection(&mut words, 144);
    let mut list = NameTagList::open(collection).unwrap();
    let mut ring = vec![0_u8; 5 * 144];
    let storage = ring.as_mut_ptr();

    // SAFETY: the ring holds room for 2 + 3 elements and is apart from the game storage
    let first_new = unsafe { list.move_to(storage, 3) };

    assert_eq!(first_new as usize, storage as usize + 2 * 144);
    let end = storage as usize + 5 * 144;
    assert_eq!(words[2..5], [storage as usize, end, end]);
    assert_eq!(ring[..2 * 144], game_storage[..]);
    assert!(ring[2 * 144..].iter().all(|byte| *byte == 0));
    assert_eq!(game_storage[0], 0xAB);
}
