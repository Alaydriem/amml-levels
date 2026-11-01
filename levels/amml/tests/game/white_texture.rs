use std::ffi::c_void;
use std::sync::Mutex;

use alaydriem_levels::game::{
    ImageBufferLayout, ImageDescriptionLayout, PathViewLayout, ResourceLocationLayout, TexturePtrLayout, WhiteTexture,
};
use alaydriem_levels::services::GameStrings;
use alaydriem_levels::types::{GameString, LevelsEntry, MeshCalls};
use amml_game::EntryId;

use crate::support::{FakeHeap, FakeMeshEntries, FakeResolver, FakeVtable, OwnImage};

// what the fake game functions saw: the uploaded pixels, the description side, the unloads and the location paths
#[derive(Default)]
struct Seen {
    uploaded: Vec<Vec<u8>>,
    side: u32,
    unloads: usize,
    paths: Vec<String>,
    // where each path the location constructor saw lies
    path_addresses: Vec<usize>,
    got_on_base: bool,
}

static SEEN: Mutex<Option<Seen>> = Mutex::new(None);
static GROUP: [u64; 8] = [0; 8];
static TEXTURE: u64 = 0;
static BLOB: Mutex<[u8; 4]> = Mutex::new([0; 4]);
// the blob size the fake ImageBuffer constructor reports
static BLOB_SIZE: Mutex<usize> = Mutex::new(4);

struct FakeGame;

impl FakeGame {
    fn seen<R>(f: impl FnOnce(&mut Seen) -> R) -> R {
        let mut guard = SEEN.lock().unwrap();
        f(guard.get_or_insert_with(Seen::default))
    }

    unsafe extern "C" fn get_group(_client: *mut c_void, out: *mut [usize; 2]) -> *mut [usize; 2] {
        // SAFETY: the wrapper passes a writable shared_ptr; a null control block holds no use
        unsafe { out.write([GROUP.as_ptr() as usize, 0]) };
        out
    }

    unsafe extern "C" fn location_ctor(
        location: *mut ResourceLocationLayout,
        view: *const PathViewLayout,
    ) -> *mut ResourceLocationLayout {
        // SAFETY: the wrapper passes a view of a live string
        let view = unsafe { *view };
        // SAFETY: as above
        let path = unsafe { std::slice::from_raw_parts(view.data, view.size) };
        Self::seen(|seen| {
            seen.paths.push(String::from_utf8_lossy(path).into_owned());
            seen.path_addresses.push(view.data as usize);
        });
        location
    }

    unsafe extern "C" fn image_ctor(
        buffer: *mut ImageBufferLayout,
        description: *const ImageDescriptionLayout,
    ) -> *mut c_void {
        // SAFETY: the wrapper passes a live description and a writable buffer
        let side = unsafe { (*description).width };
        Self::seen(|seen| seen.side = side);
        let mut blob = BLOB.lock().unwrap();
        // SAFETY: as above; the blob is a static that outlives the test
        unsafe {
            (*buffer).data = blob.as_mut_ptr();
            (*buffer).size = *BLOB_SIZE.lock().unwrap();
        }
        buffer.cast()
    }

    unsafe extern "C" fn upload(
        _group: *mut c_void,
        _location: *const ResourceLocationLayout,
        buffer: *mut ImageBufferLayout,
    ) -> *mut c_void {
        // SAFETY: the wrapper passes the buffer it built, whose blob holds `size` bytes
        let bytes = unsafe { std::slice::from_raw_parts((*buffer).data, (*buffer).size) }.to_vec();
        Self::seen(|seen| seen.uploaded.push(bytes));
        std::ptr::null_mut()
    }

    unsafe extern "C" fn get(
        base: *mut c_void,
        out: *mut [usize; 4],
        _location: *const ResourceLocationLayout,
        _reload: u8,
        _mips: u64,
        _layer: u32,
    ) -> *mut [usize; 4] {
        let on_base = base as usize == GROUP.as_ptr() as usize + 0x18;
        Self::seen(|seen| seen.got_on_base = on_base);
        // SAFETY: the wrapper passes a writable TexturePtr; null control blocks hold no use
        unsafe { out.write([&raw const TEXTURE as usize, 0, 0, 0]) };
        out
    }

    unsafe extern "C" fn unload(_group: *mut c_void, _location: *const ResourceLocationLayout, _assert: u8) -> u8 {
        Self::seen(|seen| seen.unloads += 1);
        1
    }

    fn calls() -> MeshCalls {
        let functions: [(EntryId, usize); 6] = [
            (LevelsEntry::GET_TEXTURE_GROUP, 1),
            (LevelsEntry::RESOURCE_LOCATION_CTOR, Self::location_ctor as *const () as usize),
            (LevelsEntry::IMAGE_BUFFER_CTOR, Self::image_ctor as *const () as usize),
            (LevelsEntry::TEXTURE_GROUP_UPLOAD_TEXTURE, Self::upload as *const () as usize),
            (LevelsEntry::TEXTURE_GROUP_GET_TEXTURE, Self::get as *const () as usize),
            (LevelsEntry::TEXTURE_GROUP_UNLOAD_TEXTURE, Self::unload as *const () as usize),
        ];
        let values: Vec<_> = FakeMeshEntries::values()
            .into_iter()
            .map(|(id, value)| {
                let replaced = functions.iter().find(|(entry, _)| *entry == id).map(|(_, function)| *function as u64);
                (id, replaced.unwrap_or(value))
            })
            .collect();
        MeshCalls::new(&FakeResolver::entries(&values)).unwrap()
    }

    // the texture path as the mesh factory passes it: a game-heap copy
    fn path() -> &'static str {
        GameStrings::new(FakeHeap::allocate).get(GameString::WhitePath).unwrap()
    }

    // a client whose vtable slot 1 is getTextureGroup
    fn client() -> *mut c_void {
        let vtable = FakeVtable::leak(&[Self::get_group as *const () as usize]);
        Box::leak(Box::new(vtable as usize)) as *mut usize as *mut c_void
    }
}

#[test]
fn load_uploads_one_opaque_white_pixel_under_one_name_and_draws_it_as_a_texture_ptr() {
    // SAFETY: the fake client and the fake functions have the game shapes
    let white = unsafe { WhiteTexture::load(FakeGame::client(), &FakeGame::calls(), FakeGame::path()) }.unwrap();

    let variant = white.variant();
    FakeGame::seen(|seen| {
        assert_eq!(seen.uploaded, [vec![0xFF; 4]]);
        assert_eq!(seen.side, 1);
        assert_eq!(seen.paths, [GameString::WhitePath.text()]);
        let image = OwnImage::range();
        assert!(seen.path_addresses.iter().all(|address| !image.contains(address)), "the path lies in the image");
        assert!(seen.got_on_base);
        assert_eq!(seen.unloads, 0);
    });
    assert_eq!(variant.index(), TexturePtrLayout::VARIANT_INDEX);
    assert!(!variant.storage().is_empty());
}

#[test]
fn drop_unloads_the_texture_once() {
    // SAFETY: as above
    let white = unsafe { WhiteTexture::load(FakeGame::client(), &FakeGame::calls(), FakeGame::path()) }.unwrap();

    drop(white);

    FakeGame::seen(|seen| assert_eq!(seen.unloads, 1));
}

#[test]
fn a_blob_of_another_size_uploads_nothing_and_gives_no_texture() {
    *BLOB_SIZE.lock().unwrap() = 16;

    // SAFETY: as above
    let white = unsafe { WhiteTexture::load(FakeGame::client(), &FakeGame::calls(), FakeGame::path()) };

    assert!(white.is_none());
    FakeGame::seen(|seen| assert!(seen.uploaded.is_empty()));
}
