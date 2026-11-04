// why a game layout is not the one the mod expects: the entries of the mesh path
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LayoutFault {
    #[error("{entry} did not resolve")]
    Missing { entry: &'static str },
    #[error("{entry} is {found:#x}, the mirror expects {expected:#x}")]
    Misfit { entry: &'static str, expected: u64, found: u64 },
}
