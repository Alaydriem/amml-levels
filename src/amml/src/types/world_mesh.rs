use crate::game::OwnedMesh;

/// One mesh of the world draw: a game mesh, or the stand-in the fake factory makes. Dropping a game mesh destroys
/// it, so it is dropped on the render thread only
pub enum WorldMesh {
    Game { mesh: OwnedMesh, vertices: usize },
    Fake { vertices: usize },
}

impl WorldMesh {
    // the vertex data the mesh keeps per vertex: a position, a packed colour and one UV
    const VERTEX_BYTES: u64 = 24;

    pub fn vertices(&self) -> usize {
        match self {
            Self::Game { vertices, .. } | Self::Fake { vertices } => *vertices,
        }
    }

    // the mesh object and its vertex data, for the resident memory count
    pub fn resident_bytes(&self) -> u64 {
        OwnedMesh::BYTES + self.vertices() as u64 * Self::VERTEX_BYTES
    }
}
