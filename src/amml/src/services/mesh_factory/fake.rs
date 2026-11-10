use levels_core::config::MAX_MARKERS;
use levels_core::structs::draw::ColorQuad;

use crate::services::{QuadWriter, WorldDrawer};
use crate::types::{BuildStall, MeshAttempt, WorldMesh, WorldSlot};

/// A mesh factory that touches no game object: it counts builds and keeps the draws of the last frame, so the
/// drawer's rebuild and draw order rules can be tested. The next builds can be made to fail or to find the
/// tessellator busy. The draw list holds every border mesh and every shell mesh of each marker at most, and is
/// emptied at the start of each frame
pub struct FakeMeshFactory {
    builds: u32,
    busy: u32,
    failing: u32,
    writer: QuadWriter,
    draws: Vec<(WorldSlot, [f32; 3])>,
}

impl Default for FakeMeshFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeMeshFactory {
    pub fn new() -> Self {
        let room = WorldDrawer::BORDER_MESHES + MAX_MARKERS as usize * WorldDrawer::SHELL_MESHES;
        Self { builds: 0, busy: 0, failing: 0, writer: QuadWriter::new(), draws: Vec::with_capacity(room) }
    }

    // the builds tried since construction
    pub fn builds(&self) -> u32 {
        self.builds
    }

    // the next `count` builds find the tessellator open
    pub fn busy_next(&mut self, count: u32) {
        self.busy = count;
    }

    // the next `count` builds fail
    pub fn fail_next(&mut self, count: u32) {
        self.failing = count;
    }

    // the slot and offset of each draw of the last frame, in draw order
    pub fn draws(&self) -> &[(WorldSlot, [f32; 3])] {
        &self.draws
    }

    pub fn prepare(&mut self) -> bool {
        self.draws.clear();
        true
    }

    pub fn build(&mut self, quads: &[ColorQuad]) -> MeshAttempt {
        self.builds += 1;
        let vertices = quads.len() * self.writer.vertices_per_quad();
        if self.busy > 0 {
            self.busy -= 1;
            return MeshAttempt::Busy(BuildStall::TessellatorOpen);
        }
        if self.failing > 0 {
            self.failing -= 1;
            return MeshAttempt::Failed { vertices };
        }
        MeshAttempt::Built(WorldMesh::Fake { vertices })
    }

    /// sets the sides of the quads the next builds count; true when that changes them
    pub fn set_two_sided(&mut self, two_sided: bool) -> bool {
        self.writer.set_two_sided(two_sided)
    }

    pub fn draw(&mut self, slot: WorldSlot, offset: [f32; 3]) {
        if self.draws.len() < self.draws.capacity() {
            self.draws.push((slot, offset));
        }
    }
}
