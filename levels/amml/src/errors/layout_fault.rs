// why a game layout is not the one the mod expects: the name-tag list, or the entries of the mesh path
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LayoutFault {
    #[error("the NameTagRenderObject.size entry is not the size of the mirror")]
    ElementSize,
    #[error("the vector pointers are not ordered first <= last <= end")]
    Unordered,
    #[error("the list length is not a whole number of elements")]
    Misaligned,
    #[error("{entry} did not resolve")]
    Missing { entry: &'static str },
    #[error("{entry} is {found:#x}, the mirror expects {expected:#x}")]
    Misfit { entry: &'static str, expected: u64, found: u64 },
}
