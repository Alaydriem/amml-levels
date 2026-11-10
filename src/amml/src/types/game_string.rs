use crate::types::WorldSlot;

// the texts the mod passes into game objects: the white texture's path and the debug name of each mesh slot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameString {
    WhitePath,
    LabelMesh,
    BorderMesh,
    ShellMesh,
}

impl GameString {
    pub const ALL: [GameString; 4] = [Self::WhitePath, Self::LabelMesh, Self::BorderMesh, Self::ShellMesh];

    pub fn text(self) -> &'static str {
        match self {
            Self::WhitePath => "alaydriem_levels/white",
            Self::LabelMesh => WorldSlot::Label.mesh_name(),
            Self::BorderMesh => WorldSlot::Border.mesh_name(),
            Self::ShellMesh => WorldSlot::Shell.mesh_name(),
        }
    }

    pub fn mesh(slot: WorldSlot) -> Self {
        match slot {
            WorldSlot::Label => Self::LabelMesh,
            WorldSlot::Border => Self::BorderMesh,
            WorldSlot::Shell => Self::ShellMesh,
        }
    }
}
