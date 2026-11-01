use std::ffi::c_void;

use alaydriem_levels::game::{GameMirror, NameTagCollection, NameTagRenderObject};
use alaydriem_levels::types::LevelsEntry;

use crate::support::FakeResolver;

// the `NameTagRenderObject.size` both game builds derived: 26.51 (data/6aa482fd-2658118eba0dbac5.hcl) and 26.60
// (data/6ad79ee4-17d51891a1119914.hcl)
const BUILDS: [(&str, u64); 2] = [("26.51", 0x90), ("26.60", 0x90)];

#[test]
fn the_list_steps_by_the_derived_size_of_each_build() {
    for (name, size) in BUILDS {
        let entries = FakeResolver::entries(&[(LevelsEntry::NAME_TAG_SIZE, size)]);
        let mut words = [0_usize; 5];

        // SAFETY: the array outlives the wrapper and is as large as the collection mirror
        let collection = unsafe { NameTagCollection::from_raw((&raw mut words).cast::<c_void>(), &entries) }.unwrap();

        assert!(NameTagRenderObject::fits(&entries), "{name}");
        assert_eq!(collection.element_size().map(|size| size as u64), Some(size), "{name}");
    }
}

#[test]
fn a_size_entry_that_is_not_the_mirror_size_gives_no_element_size() {
    let entries = FakeResolver::entries(&[(LevelsEntry::NAME_TAG_SIZE, 0x98)]);
    let mut words = [0_usize; 5];

    // SAFETY: as above
    let collection = unsafe { NameTagCollection::from_raw((&raw mut words).cast::<c_void>(), &entries) }.unwrap();

    assert_eq!(collection.element_size(), None);
    assert_eq!(
        NameTagRenderObject::misfit(&entries).as_deref(),
        Some("NameTagRenderObject.size is 0x98, the mirror is 0x90")
    );
}
