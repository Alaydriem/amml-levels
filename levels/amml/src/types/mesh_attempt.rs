use crate::types::{BuildStall, WorldMesh};

// what one mesh build gave
pub enum MeshAttempt {
    // the build could not run; it is tried again next frame
    Busy(BuildStall),
    Built(WorldMesh),
    // the fill failed or the end made an empty mesh; `vertices` is what the tessellator held before the end
    Failed { vertices: usize },
}
