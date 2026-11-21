// the three mesh slots of the world draw
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldSlot {
    Label,
    Border,
    Shell,
}

impl WorldSlot {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Label => "label",
            Self::Border => "border",
            Self::Shell => "shell",
        }
    }
}
