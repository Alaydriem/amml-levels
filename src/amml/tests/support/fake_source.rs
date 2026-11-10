use std::ffi::c_void;

use amml_game::BlockPos;

use super::{FakeChunk, FakeVtable};

// a fake `BlockSource` with one chunk: `getChunk` in slot 1, `getBrightnessPair` as a function entry. It records the
// chunk coordinates asked for and every position whose light was read
#[repr(C)]
pub struct FakeSource {
    pub vtable: *const usize,
    pub chunk: *mut FakeChunk,
    pub light: [u8; 2],
    pub chunk_asks: Vec<[i32; 2]>,
    pub light_reads: Vec<BlockPos>,
}

impl FakeSource {
    pub const GET_CHUNK_SLOT: u64 = 1;

    pub fn new(chunk: *mut FakeChunk, light: [u8; 2]) -> Box<Self> {
        let get_chunk: unsafe extern "C" fn(*mut c_void, i32, i32) -> *mut c_void = Self::get_chunk;
        Box::new(Self {
            vtable: FakeVtable::leak(&[get_chunk as usize]),
            chunk,
            light,
            chunk_asks: Vec::new(),
            light_reads: Vec::new(),
        })
    }

    pub fn ptr(&mut self) -> *mut c_void {
        (&raw mut *self).cast()
    }

    unsafe extern "C" fn get_chunk(this: *mut c_void, x: i32, z: i32) -> *mut c_void {
        // SAFETY: the fake is only reached through a FakeSource
        let source = unsafe { &mut *this.cast::<FakeSource>() };
        source.chunk_asks.push([x, z]);
        source.chunk.cast()
    }

    // `BrightnessPair BlockSource::getBrightnessPair(BlockPos const&) const`, through the hidden return pointer
    pub unsafe extern "C" fn get_brightness_pair(
        this: *mut c_void,
        out: *mut [u8; 2],
        pos: *const BlockPos,
    ) -> *mut [u8; 2] {
        // SAFETY: the fake is only reached through a FakeSource, with the caller buffer and position
        unsafe {
            let source = &mut *this.cast::<FakeSource>();
            source.light_reads.push(*pos);
            *out = source.light;
        }
        out
    }
}
