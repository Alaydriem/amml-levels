use amml_msvc::MsvcHeap;
use levels_core::structs::draw::ColorQuad;

use crate::game::{GameVec, Tessellator, WhiteTexture};

/// Writes core's `ColorQuad`s into the mesh data of an open quad-list begin: positions, packed colours and the first
/// UV set (a point inside the white texture), then `mCount`. One-sided, each quad goes in once, its corners in
/// order, for a material that draws both sides (`DisableCulling`). Two-sided, each quad goes in a second time with
/// its corners reversed, for a material that culls back faces. The vectors grow on the game heap; the writer keeps
/// no buffer of its own
#[derive(Default)]
pub struct QuadWriter {
    two_sided: bool,
}

impl QuadWriter {
    // the vertices one quad takes one-sided: its four corners
    pub const VERTICES_PER_QUAD: usize = 4;
    // the most vertices one quad takes: two-sided, its four corners and the same four reversed
    pub const MAX_VERTICES_PER_QUAD: usize = 2 * Self::VERTICES_PER_QUAD;

    // a one-sided writer
    pub fn new() -> Self {
        Self { two_sided: false }
    }

    /// sets the sides; true when that changes what the writer writes
    pub fn set_two_sided(&mut self, two_sided: bool) -> bool {
        let changed = self.two_sided != two_sided;
        self.two_sided = two_sided;
        changed
    }

    // the vertices one quad takes with the current sides
    pub fn vertices_per_quad(&self) -> usize {
        if self.two_sided { Self::MAX_VERTICES_PER_QUAD } else { Self::VERTICES_PER_QUAD }
    }

    /// the game's packed `mce::Color` of a vertex: red in the low byte, alpha in the high one (ABGR as a `u32`)
    pub fn pack(rgba: [u8; 4]) -> u32 {
        u32::from_le_bytes(rgba)
    }

    // the quad and the corner of vertex `index`, for `per` vertices a quad; the second four go in reverse order
    fn corner(index: usize, per: usize) -> (usize, usize) {
        let quad = index / per;
        let step = index % per;
        let corner = if step < Self::VERTICES_PER_QUAD { step } else { per - 1 - step };
        (quad, corner)
    }

    /// Appends the vertices of `quads` and sets `mCount` to the vertex total. False when a vector could not grow
    /// or the total does not fit `mCount`; the caller then discards the begin
    ///
    /// # Safety
    /// `heap` must be the game's allocator, the one the tessellator's vectors came from
    #[allow(unsafe_code)]
    pub unsafe fn write(&mut self, tessellator: &mut Tessellator, heap: &MsvcHeap, quads: &[ColorQuad]) -> bool {
        let per = self.vertices_per_quad();
        let count = quads.len() * per;
        let data = tessellator.mesh_data();
        // SAFETY: the caller guarantees the game's allocator, and the vectors are the game's
        let written = unsafe {
            GameVec::extend_with(heap, &mut data.positions, count, |index| {
                let (quad, corner) = Self::corner(index, per);
                quads[quad].corners[corner]
            })
            .and_then(|()| {
                GameVec::extend_with(heap, &mut data.colors, count, |index| Self::pack(quads[index / per].rgba))
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
