use std::ffi::c_void;
use std::mem::MaybeUninit;

use alaydriem_levels::game::{Tessellator, TessellatorLayout};
use alaydriem_levels::services::QuadWriter;
use alaydriem_levels::types::MeshCalls;
use levels_core::structs::draw::ColorQuad;

use crate::support::{FakeHeap, FakeMeshEntries};

const QUAD: ColorQuad = ColorQuad {
    corners: [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
    rgba: [0x50, 0xA0, 0xFF, 0x60],
};

// a zeroed tessellator: every vector empty, no begin open
fn zeroed() -> Box<MaybeUninit<TessellatorLayout>> {
    Box::new(MaybeUninit::zeroed())
}

#[test]
fn packs_rgba_as_abgr() {
    assert_eq!(QuadWriter::pack([0x50, 0xA0, 0xFF, 0x60]), 0x60FF_A050);
}

#[test]
fn writes_eight_vertices_per_quad_in_both_windings_and_sets_the_count() {
    let calls: MeshCalls = FakeMeshEntries::calls();
    let heap = FakeHeap::msvc();
    let mut layout = zeroed();
    let other = ColorQuad { corners: [[5.0, 0.0, 5.0]; 4], rgba: [1, 2, 3, 4] };
    // SAFETY: a zeroed layout is a tessellator with empty vectors; it outlives the wrapper
    let mut tessellator = unsafe { Tessellator::from_raw(layout.as_mut_ptr().cast::<c_void>(), &calls) }.unwrap();

    // SAFETY: the fake heap allocates the vectors the writer grows
    let written = unsafe { QuadWriter::new().write(&mut tessellator, &heap, &[QUAD, other]) };

    assert!(written);
    assert_eq!(tessellator.count(), 16);
    let data = tessellator.mesh_data();
    let [a, b, c, d] = QUAD.corners;
    assert_eq!(&data.positions.as_slice()[..8], [a, b, c, d, d, c, b, a]);
    assert_eq!(&data.positions.as_slice()[8..], [[5.0, 0.0, 5.0]; 8]);
    assert_eq!(&data.colors.as_slice()[..8], [0x60FF_A050; 8]);
    assert_eq!(&data.colors.as_slice()[8..], [0x0403_0201; 8]);
    assert_eq!(data.texture_uvs[0].len(), 16);
    assert!(data.normals.is_empty());
    // SAFETY: the vectors came from the fake heap and are not used again
    unsafe {
        data.positions.release(&heap);
        data.colors.release(&heap);
        data.texture_uvs[0].release(&heap);
    }
}

#[test]
fn a_second_write_appends_after_the_vertices_already_written() {
    let calls: MeshCalls = FakeMeshEntries::calls();
    let heap = FakeHeap::msvc();
    let mut layout = zeroed();
    // SAFETY: as above
    let mut tessellator = unsafe { Tessellator::from_raw(layout.as_mut_ptr().cast::<c_void>(), &calls) }.unwrap();
    let mut writer = QuadWriter::new();

    // SAFETY: as above
    let written =
        unsafe { writer.write(&mut tessellator, &heap, &[QUAD]) && writer.write(&mut tessellator, &heap, &[QUAD; 3]) };

    assert!(written);
    assert_eq!(tessellator.count(), 32);
    assert_eq!(tessellator.vertex_count(), 32);
    let data = tessellator.mesh_data();
    assert_eq!(data.colors.len(), 32);
    // SAFETY: as above
    unsafe {
        data.positions.release(&heap);
        data.colors.release(&heap);
        data.texture_uvs[0].release(&heap);
    }
}
