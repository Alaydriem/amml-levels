use amml_msvc::MsvcHeap;
use levels_core::structs::draw::ColorQuad;

use crate::game::{GameVec, Tessellator, WhiteTexture};

/// Writes core's `ColorQuad`s into the mesh data of an open quad-list begin: positions, packed colours and the first
/// UV set (a point inside the white texture), then `mCount`. Each quad goes in twice, the second time with its corners
/// reversed, so it is drawn from both sides past back-face culling. The vectors grow on the game heap; the writer
/// keeps no buffer of its own
#[derive(Default)]
pub struct QuadWriter;

impl QuadWriter {
    // the vertices one quad takes: its four corners, then the same four in reverse order
    pub const VERTICES_PER_QUAD: usize = 8;

    pub fn new() -> Self {
        Self
    }

    /// the game's packed `mce::Color` of a vertex: red in the low byte, alpha in the high one (ABGR as a `u32`)
    pub fn pack(rgba: [u8; 4]) -> u32 {
        u32::from_le_bytes(rgba)
    }

    // the quad and the corner of vertex `index`
    fn corner(index: usize) -> (usize, usize) {
        let quad = index / Self::VERTICES_PER_QUAD;
        let step = index % Self::VERTICES_PER_QUAD;
        let corner = if step < 4 { step } else { Self::VERTICES_PER_QUAD - 1 - step };
        (quad, corner)
    }

    /// Appends the vertices of `quads` and sets `mCount` to the vertex total. False when a vector could not grow
    /// or the total does not fit `mCount`; the caller then discards the begin
    ///
    /// # Safety
    /// `heap` must be the game's allocator, the one the tessellator's vectors came from
    #[allow(unsafe_code)]
    pub unsafe fn write(&mut self, tessellator: &mut Tessellator, heap: &MsvcHeap, quads: &[ColorQuad]) -> bool {
        let count = quads.len() * Self::VERTICES_PER_QUAD;
        let data = tessellator.mesh_data();
        // SAFETY: the caller guarantees the game's allocator, and the vectors are the game's
        let written = unsafe {
            GameVec::extend_with(heap, &mut data.positions, count, |index| {
                let (quad, corner) = Self::corner(index);
                quads[quad].corners[corner]
            })
            .and_then(|()| {
                GameVec::extend_with(heap, &mut data.colors, count, |index| {
                    Self::pack(quads[index / Self::VERTICES_PER_QUAD].rgba)
                })
            })
            .and_then(|()| GameVec::extend_with(heap, &mut data.texture_uvs[0], count, |_| WhiteTexture::UV))
        };
        if written.is_err() {
            return false;
        }
        let Ok(total) = u32::try_from(tessellator.vertex_count()) else { return false };
        tessellator.set_count(total);
        true
    }
}
