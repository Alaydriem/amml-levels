use crate::game::OwnedMesh;

/// What one begin, fill and end on the game's tessellator gave
pub struct MeshBuild {
    /// what the fill step returned
    pub filled: bool,
    /// the vertices the tessellator held before the end
    pub vertices: usize,
    /// the mesh the end made; `None` when the end made none or the fill failed
    pub mesh: Option<OwnedMesh>,
}
