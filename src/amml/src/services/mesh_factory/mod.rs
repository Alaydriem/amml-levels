pub mod fake;
pub mod game;

pub use fake::FakeMeshFactory;
pub use game::GameMeshFactory;

use levels_core::structs::draw::ColorQuad;

use crate::types::{MeshAttempt, WorldFrame, WorldMesh, WorldSlot};

/// Where the world drawer's meshes come from: the game, or a fake for the tests
// one value per process, so the size of the game variant costs nothing
#[allow(clippy::large_enum_variant)]
pub enum MeshFactory {
    Game(GameMeshFactory),
    Fake(FakeMeshFactory),
}

impl MeshFactory {
    /// Gets the factory ready for a frame; false when it cannot draw this frame
    ///
    /// # Safety
    /// `frame` must hold the live screen and client of the running render call. Render thread only
    #[allow(unsafe_code)]
    pub unsafe fn prepare(&mut self, frame: &WorldFrame<'_>) -> bool {
        match self {
            // SAFETY: forwarded to the caller
            Self::Game(factory) => unsafe { factory.prepare(frame) },
            Self::Fake(factory) => factory.prepare(),
        }
    }

    /// # Safety
    /// As for `prepare`
    #[allow(unsafe_code)]
    pub unsafe fn build(&mut self, frame: &WorldFrame<'_>, quads: &[ColorQuad], slot: WorldSlot) -> MeshAttempt {
        match self {
            // SAFETY: forwarded to the caller
            Self::Game(factory) => unsafe { factory.build(frame, quads, slot) },
            Self::Fake(factory) => factory.build(quads),
        }
    }

    /// # Safety
    /// As for `prepare`; `frame.material` must be null or a live `mce::MaterialPtr`
    #[allow(unsafe_code)]
    pub unsafe fn draw(
        &mut self,
        frame: &WorldFrame<'_>,
        slot: WorldSlot,
        mesh: &WorldMesh,
        offset: [f32; 3],
        yaw_degrees: f32,
    ) {
        match self {
            // SAFETY: forwarded to the caller
            Self::Game(factory) => unsafe { factory.draw(frame, mesh, offset, yaw_degrees) },
            Self::Fake(factory) => factory.draw(slot, offset),
        }
    }

    /// sets the sides of the quads the next builds write; true when that changes them
    pub fn set_two_sided(&mut self, two_sided: bool) -> bool {
        match self {
            Self::Game(factory) => factory.set_two_sided(two_sided),
            Self::Fake(factory) => factory.set_two_sided(two_sided),
        }
    }

    // gives back what the factory holds for drawing; the meshes must be dropped first
    pub fn release(&mut self) {
        match self {
            Self::Game(factory) => factory.release(),
            Self::Fake(_) => {}
        }
    }
}
