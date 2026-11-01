use std::ffi::c_void;
use std::mem::MaybeUninit;

use amml_game::{ChunkBlockPos, LevelChunkLayout2651};

// a fake `LevelChunk` laid out as the 26.51 mirror, with the test state after it. `getBlock` gives `blocks[y]` for a
// y relative to the minimum and `air` past the table, and records every position it was asked for
#[repr(C)]
pub struct FakeChunk {
    pub layout: LevelChunkLayout2651,
    pub blocks: Vec<*mut c_void>,
    pub air: *mut c_void,
    pub reads: Vec<ChunkBlockPos>,
}

impl FakeChunk {
    pub const LOADED: u8 = 11;
    // the `LevelChunk.mSubChunks` value that selects the 26.51 mirror
    pub const SUB_CHUNKS_AT: u64 = LevelChunkLayout2651::SUB_CHUNKS;

    pub fn new(min_y: i32, max_y: i32, blocks: Vec<*mut c_void>, air: *mut c_void) -> Box<Self> {
        // SAFETY: the mirror holds integers, raw pointers and byte spans, for which all zero bytes are valid
        let mut layout: LevelChunkLayout2651 = unsafe { MaybeUninit::zeroed().assume_init() };
        layout.min = [0, min_y, 0];
        layout.max = [15, max_y, 15];
        layout.load_state = Self::LOADED;
        Box::new(Self { layout, blocks, air, reads: Vec::new() })
    }

    // `Block const& LevelChunk::getBlock(ChunkBlockPos const&) const`
    pub unsafe extern "C" fn get_block(this: *mut c_void, pos: *const ChunkBlockPos) -> *mut c_void {
        // SAFETY: the sampler calls this with a FakeChunk and a live position
        let (chunk, pos) = unsafe { (&mut *this.cast::<FakeChunk>(), *pos) };
        chunk.reads.push(pos);
        usize::try_from(pos.y).ok().and_then(|y| chunk.blocks.get(y).copied()).unwrap_or(chunk.air)
    }
}
