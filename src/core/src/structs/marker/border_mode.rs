#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BorderMode {
    #[default]
    Off,
    Single,
    Grid,
}

impl BorderMode {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Single,
            Self::Single => Self::Grid,
            Self::Grid => Self::Off,
        }
    }
}
