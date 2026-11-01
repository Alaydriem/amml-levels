use crate::types::WorldSlot;

// the texts the mod passes into game objects: the white texture's path and the debug name of each mesh slot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameString {
    WhitePath,
    BorderMesh,
    ShellMesh,
}

impl GameString {
    pub const ALL: [GameString; 3] = [Self::WhitePath, Self::BorderMesh, Self::ShellMesh];

    pub fn text(self) -> &'static str {
        match self {
            Self::WhitePath => "alaydriem_levels/white",
            Self::BorderMesh => WorldSlot::Border.mesh_name(),
            Self::ShellMesh => WorldSlot::Shell.mesh_name(),
        }
    }

    pub fn mesh(slot: WorldSlot) -> Self {
        match slot {
            WorldSlot::Border => Self::BorderMesh,
            WorldSlot::Shell => Self::ShellMesh,
        }
    }
}
