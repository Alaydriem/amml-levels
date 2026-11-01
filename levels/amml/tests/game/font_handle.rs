use alaydriem_levels::game::{FontHandle, GameMirror};
use alaydriem_levels::types::LevelsEntry;

use crate::support::FakeResolver;

// the values both game builds derived for the members the mirror now holds: 26.51
// (data/6aa482fd-2658118eba0dbac5.hcl) and 26.60 (data/6ad79ee4-17d51891a1119914.hcl)
struct Build {
    name: &'static str,
    handle_size: u64,
    repository_at: usize,
    default_font_at: usize,
    font_id_at: usize,
    fonts_begin_at: usize,
    fonts_end_at: usize,
    element_shift: u32,
}

const BUILDS: [Build; 2] = [
    Build {
        name: "26.51",
        handle_size: 0x50,
        repository_at: 0x28,
        default_font_at: 0x30,
        font_id_at: 0x40,
        fonts_begin_at: 0x40,
        fonts_end_at: 0x48,
        element_shift: 4,
    },
    Build {
        name: "26.60",
        handle_size: 0x50,
        repository_at: 0x28,
        default_font_at: 0x30,
        font_id_at: 0x40,
        fonts_begin_at: 0x40,
        fonts_end_at: 0x48,
        element_shift: 4,
    },
];

const DEFAULT: u64 = 0xD0;
const FIRST: u64 = 0xA1;
const SECOND: u64 = 0xA2;

// a handle, its repository and two fonts in memory laid out at the derived offsets of one build
struct Fixture {
    handle: Vec<u64>,
    repository: Vec<u64>,
    fonts: Vec<u64>,
}

impl Fixture {
    fn new(build: &Build, id: u64) -> Self {
        let element_words = (1_usize << build.element_shift) / 8;
        let mut fonts = vec![0; 2 * element_words];
        fonts[0] = FIRST;
        fonts[element_words] = SECOND;
        let mut fixture = Self { handle: vec![0; build.handle_size as usize / 8], repository: vec![0; 16], fonts };
        fixture.repository[build.fonts_begin_at / 8] = fixture.fonts.as_ptr() as u64;
        fixture.repository[build.fonts_end_at / 8] = fixture.fonts.as_ptr() as u64 + (2 << build.element_shift);
        fixture.handle[build.repository_at / 8] = fixture.repository.as_ptr() as u64;
        fixture.handle[build.default_font_at / 8] = DEFAULT;
        fixture.handle[build.font_id_at / 8] = id;
        fixture
    }

    fn font(&self) -> Option<u64> {
        // SAFETY: the handle buffer is the size of the mirror, aligned to 8, and its repository and fonts are live
        unsafe { (*self.handle.as_ptr().cast::<FontHandle>()).font() }.map(|font| font as u64)
    }
}

#[test]
fn the_mirror_has_the_derived_size_of_each_build() {
    for build in &BUILDS {
        let entries = FakeResolver::entries(&[(LevelsEntry::FONT_HANDLE_SIZE, build.handle_size)]);

        assert!(FontHandle::fits(&entries), "{}", build.name);
        assert_eq!(FontHandle::misfit(&entries), None, "{}", build.name);
    }
}

#[test]
fn a_valid_id_picks_the_font_at_that_index_of_the_repository_on_each_build() {
    for build in &BUILDS {
        assert_eq!(Fixture::new(build, 0).font(), Some(FIRST), "{}", build.name);
        assert_eq!(Fixture::new(build, 1).font(), Some(SECOND), "{}", build.name);
    }
}

#[test]
fn the_invalid_id_and_an_id_past_the_end_pick_the_default_font() {
    for build in &BUILDS {
        assert_eq!(Fixture::new(build, u64::MAX).font(), Some(DEFAULT), "{}", build.name);
        assert_eq!(Fixture::new(build, 2).font(), Some(DEFAULT), "{}", build.name);
    }
}

#[test]
fn without_a_repository_or_with_an_empty_vector_the_default_font_is_picked() {
    let build = &BUILDS[0];
    let mut fixture = Fixture::new(build, 0);
    fixture.handle[build.repository_at / 8] = 0;
    assert_eq!(fixture.font(), Some(DEFAULT));

    let mut fixture = Fixture::new(build, 0);
    fixture.repository[build.fonts_end_at / 8] = fixture.repository[build.fonts_begin_at / 8];
    assert_eq!(fixture.font(), Some(DEFAULT));

    let mut fixture = Fixture::new(build, 0);
    fixture.repository[build.fonts_begin_at / 8] = 0;
    assert_eq!(fixture.font(), Some(DEFAULT));
}

#[test]
fn no_font_at_all_is_none() {
    let build = &BUILDS[0];
    let mut fixture = Fixture::new(build, u64::MAX);
    fixture.handle[build.default_font_at / 8] = 0;
    assert_eq!(fixture.font(), None);

    fixture.handle[build.repository_at / 8] = 0;
    assert_eq!(fixture.font(), None);
}

#[test]
fn a_size_entry_that_is_not_the_mirror_size_does_not_fit() {
    let entries = FakeResolver::entries(&[(LevelsEntry::FONT_HANDLE_SIZE, 0x58)]);

    assert!(!FontHandle::fits(&entries));
    assert_eq!(FontHandle::misfit(&entries).as_deref(), Some("FontHandle.size is 0x58, the mirror is 0x50"));
}

#[test]
fn a_missing_size_entry_does_not_fit_and_names_no_size() {
    let entries = FakeResolver::entries(&[]);

    assert!(!FontHandle::fits(&entries));
    assert_eq!(FontHandle::misfit(&entries), None);
}
