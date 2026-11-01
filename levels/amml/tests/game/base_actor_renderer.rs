use std::sync::atomic::{AtomicUsize, Ordering};

use alaydriem_levels::game::{BaseActorRenderer, GameMirror};
use alaydriem_levels::types::LevelsEntry;
use amml_game::GameEntries;

use crate::support::{FakeHeap, FakeResolver};

// the values both game builds derived (name, `BaseActorRenderer.size`, then the depth-tested tag and text materials,
// which the mirror now holds): 26.51 (data/6aa482fd-2658118eba0dbac5.hcl) and 26.60
// (data/6ad79ee4-17d51891a1119914.hcl)
const BUILDS: [(&str, u64, usize, usize); 2] = [("26.51", 0xB8, 0x78, 0x98), ("26.60", 0xB8, 0x78, 0x98)];

static CONSTRUCTED: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn fake_ctor(this: *mut u8) -> *mut u8 {
    CONSTRUCTED.fetch_add(1, Ordering::SeqCst);
    this
}

fn entries(size: u64) -> GameEntries {
    FakeResolver::entries(&[
        (LevelsEntry::BASE_ACTOR_RENDERER_SIZE, size),
        (LevelsEntry::BASE_ACTOR_RENDERER_CTOR, fake_ctor as unsafe extern "C" fn(*mut u8) -> *mut u8 as usize as u64),
    ])
}

#[test]
fn the_materials_sit_at_the_derived_offsets_of_each_build() {
    for (name, size, tag_at, text_at) in BUILDS {
        let entries = entries(size);

        // SAFETY: the fake heap and the fake constructor have the game shapes; the renderer is never freed
        let renderer = unsafe { BaseActorRenderer::create(&entries, FakeHeap::allocate) }.unwrap();

        assert!(BaseActorRenderer::fits(&entries), "{name}");
        assert_eq!(renderer.tag_material() as usize - renderer.as_address(), tag_at, "{name}");
        assert_eq!(renderer.text_material() as usize - renderer.as_address(), text_at, "{name}");
    }
}

#[test]
fn the_renderer_is_found_again_at_its_address() {
    let entries = entries(0xB8);
    // SAFETY: as above
    let renderer = unsafe { BaseActorRenderer::create(&entries, FakeHeap::allocate) }.unwrap();

    // SAFETY: the address came from `create`
    let again = unsafe { BaseActorRenderer::from_address(renderer.as_address()) }.unwrap();

    assert_eq!(again.tag_material(), renderer.tag_material());
}

#[test]
fn a_size_entry_that_is_not_the_mirror_size_makes_no_renderer() {
    CONSTRUCTED.store(0, Ordering::SeqCst);
    let entries = entries(0xC0);

    // SAFETY: as above
    let renderer = unsafe { BaseActorRenderer::create(&entries, FakeHeap::allocate) };

    assert!(renderer.is_none());
    assert_eq!(CONSTRUCTED.load(Ordering::SeqCst), 0);
    assert_eq!(
        BaseActorRenderer::misfit(&entries).as_deref(),
        Some("BaseActorRenderer.size is 0xc0, the mirror is 0xb8")
    );
}
