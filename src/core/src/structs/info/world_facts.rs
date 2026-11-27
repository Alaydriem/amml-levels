use crate::structs::info::InfoConnection;

// the world facts read once per world; the name is copied into a fixed buffer and not kept
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InfoWorldFacts<'a> {
    pub level_name: Option<&'a str>,
    pub seed: Option<u64>,
    pub connection: InfoConnection,
}
