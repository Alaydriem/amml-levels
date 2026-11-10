use std::ffi::c_void;

use alaydriem_levels::services::SurfaceSampler;
use alaydriem_levels::types::SampleOutcome;
use amml_game::{BlockSource, EntryId};
use levels_core::structs::frame::ColumnRequest;
use levels_core::structs::world::{BlockPos, ColumnPos};

use crate::support::{FakeBlock, FakeBlockType, FakeChunk, FakeResolver, FakeSource};

// stone at y -64 to 62, air above; the chunk spans -64 to 320
struct World {
    entries: amml_game::GameEntries,
    source: Box<FakeSource>,
    chunk: Box<FakeChunk>,
    _stone: Box<FakeBlock>,
    _air: Box<FakeBlock>,
    _stone_type: Box<FakeBlockType>,
    _air_type: Box<FakeBlockType>,
}

impl World {
    const MIN_Y: i32 = -64;
    const MAX_Y: i32 = 320;
    const TOP_STONE: i32 = 62;

    fn new(state: u8) -> Self {
        let mut stone_type = FakeBlockType::new("minecraft:stone", [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        let mut air_type = FakeBlockType::new("minecraft:air", [0.0; 6]);
        let mut stone = FakeBlock::new(&raw mut *stone_type, true);
        let mut air = FakeBlock::new(&raw mut *air_type, false);
        let stone_ptr = stone.ptr();
        let air_ptr = air.ptr();
        let height = usize::try_from(Self::TOP_STONE - Self::MIN_Y + 1).unwrap();
        let mut chunk = FakeChunk::new(Self::MIN_Y, Self::MAX_Y, vec![stone_ptr; height], air_ptr);
        chunk.layout.load_state = state;
        let source = FakeSource::new(&raw mut *chunk, [3, 14]);
        let entries = FakeResolver::entries(&[
            (EntryId::BLOCK_SOURCE_GET_CHUNK, FakeSource::GET_CHUNK_SLOT),
            (EntryId::LEVEL_CHUNK_SUB_CHUNKS, FakeChunk::SUB_CHUNKS_AT),
            (
                EntryId::LEVEL_CHUNK_GET_BLOCK,
                FakeChunk::get_block as unsafe extern "C" fn(*mut c_void, *const _) -> *mut c_void as usize as u64,
            ),
            (EntryId::BLOCK_TYPE_GET_COLLISION_SHAPE, FakeBlockType::SHAPE_SLOT),
            (
                EntryId::BLOCK_CAN_PROVIDE_FULL_SUPPORT,
                FakeBlock::can_provide_full_support as unsafe extern "C" fn(*mut c_void, u8) -> u8 as usize as u64,
            ),
            (
                EntryId::BLOCK_SOURCE_GET_BRIGHTNESS_PAIR,
                FakeSource::get_brightness_pair
                    as unsafe extern "C" fn(*mut c_void, *mut [u8; 2], *const amml_game::BlockPos) -> *mut [u8; 2]
                    as usize as u64,
            ),
        ]);
        Self { entries, source, chunk, _stone: stone, _air: air, _stone_type: stone_type, _air_type: air_type }
    }

    fn sample(&mut self, request: &ColumnRequest) -> (SampleOutcome, Vec<levels_core::structs::light::SurfaceSample>) {
        // SAFETY: the fake source is live and has the layout the entries name
        let region = unsafe { BlockSource::from_raw(self.source.ptr(), &self.entries) }.unwrap();
        let mut out = Vec::new();
        let outcome = SurfaceSampler::sample(&region, request, &mut out);
        (outcome, out)
    }
}

fn request(y_min: i32, y_max: i32) -> ColumnRequest {
    ColumnRequest { column: ColumnPos { x: 5, z: -3 }, y_min, y_max }
}

#[test]
fn the_top_of_the_stone_is_the_one_surface_and_carries_the_light_above_it() {
    let mut world = World::new(FakeChunk::LOADED);

    let (outcome, out) = world.sample(&request(50, 70));

    assert_eq!(outcome, SampleOutcome::Sampled(1));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].pos, BlockPos { x: 5, y: World::TOP_STONE, z: -3 });
    assert_eq!((out[0].sky_light, out[0].block_light), (3, 14));
    assert_eq!(world.source.light_reads, [amml_game::BlockPos { x: 5, y: World::TOP_STONE + 1, z: -3 }]);
}

#[test]
fn the_chunk_is_looked_up_once_for_the_column() {
    let mut world = World::new(FakeChunk::LOADED);

    world.sample(&request(0, 100));

    // block x 5 and z -3 are chunk (0, -1)
    assert_eq!(world.source.chunk_asks, [[0, -1]]);
}

#[test]
fn a_chunk_that_is_not_loaded_submits_nothing_and_reads_nothing() {
    let mut world = World::new(10);

    let (outcome, out) = world.sample(&request(-64, 320));

    assert_eq!(outcome, SampleOutcome::Skipped);
    assert!(out.is_empty());
    assert!(world.chunk.reads.is_empty());
    assert!(world.source.light_reads.is_empty());
}

#[test]
fn no_block_is_read_outside_the_chunk_bounds() {
    let mut world = World::new(FakeChunk::LOADED);

    world.sample(&request(-1000, 1000));

    let ys: Vec<i32> = world.chunk.reads.iter().map(|pos| i32::from(pos.y) + World::MIN_Y).collect();
    assert!(!ys.is_empty());
    assert!(ys.iter().all(|y| (World::MIN_Y..World::MAX_Y).contains(y)), "{ys:?}");
    assert!(!ys.contains(&World::MAX_Y));
    assert!(!ys.contains(&(World::MIN_Y - 1)));
    assert!(world.source.light_reads.iter().all(|pos| (World::MIN_Y..World::MAX_Y).contains(&pos.y)));
}

#[test]
fn a_request_outside_the_chunk_range_samples_nothing() {
    let mut world = World::new(FakeChunk::LOADED);

    let (outcome, out) = world.sample(&request(400, 500));

    assert_eq!(outcome, SampleOutcome::Sampled(0));
    assert!(out.is_empty());
    assert!(world.chunk.reads.is_empty());
}
