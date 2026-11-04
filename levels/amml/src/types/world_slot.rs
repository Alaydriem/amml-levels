// the three mesh slots of the world draw
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldSlot {
    Label,
    Border,
    Shell,
}

impl WorldSlot {
    // the debug name of each mesh of the slot; the game gets a game-heap copy (`GameStrings`)
    pub fn mesh_name(self) -> &'static str {
        match self {
            Self::Label => "levels_label",
            Self::Border => "levels_border",
            Self::Shell => "levels_shell",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Label => "label",
            Self::Border => "border",
            Self::Shell => "shell",
        }
    }
}
