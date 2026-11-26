// names core cannot compute; they are copied into the lines and not kept
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InfoNames<'a> {
    pub biome: Option<&'a str>,
    pub target_block: Option<&'a str>,
}
