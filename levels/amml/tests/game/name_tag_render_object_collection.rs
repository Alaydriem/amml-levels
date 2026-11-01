use std::ffi::c_void;

use alaydriem_levels::game::{NameTagCollection, NameTagRenderObjectCollection};
use alaydriem_levels::types::LevelsEntry;

use crate::support::FakeResolver;

// the `NameTagRenderObjectCollection.nameTagsFirst` value both game builds derived before it was folded into the
// mirror: 26.51 (data/6aa482fd-2658118eba0dbac5.hcl) and 26.60 (data/6ad79ee4-17d51891a1119914.hcl)
const BUILDS: [(&str, usize); 2] = [("26.51", 0x10), ("26.60", 0x10)];

#[test]
fn the_vector_pointers_are_read_and_written_at_the_derived_offset_of_each_build() {
    for (name, first_at) in BUILDS {
        let entries = FakeResolver::entries(&[(LevelsEntry::NAME_TAG_SIZE, 0x90)]);
        let mut words = [0x5A5A_usize; 5];
        let at = first_at / 8;
        words[at..at + 3].copy_from_slice(&[0x1000, 0x1120, 0x1240]);

        // SAFETY: the array outlives the wrapper and is as large as the collection mirror
        let collection = unsafe { NameTagCollection::from_raw((&raw mut words).cast::<c_void>(), &entries) }.unwrap();
        let read = collection.pointers();
        collection.set_pointers([0x2000, 0x2090, 0x2120]);

        assert_eq!(read, [0x1000, 0x1120, 0x1240], "{name}");
        assert_eq!(words[at..at + 3], [0x2000, 0x2090, 0x2120], "{name}");
        assert!(words[..at].iter().all(|word| *word == 0x5A5A), "{name}: the allocator is not written");
        assert_eq!(size_of::<NameTagRenderObjectCollection>(), 40, "{name}");
    }
}
