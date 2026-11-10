use amml_game::{EntryId, GameEntries};

/// A `#[repr(C)]` mirror of a game class, laid out from the 26.51 header. The code holds its size (it allocates or
/// steps over the class), so the size entry is the check that the running build still has the mirrored layout
pub trait GameMirror: Sized {
    const SIZE_ENTRY: EntryId;

    /// whether the size entry resolved and is the mirror size
    fn fits(entries: &GameEntries) -> bool {
        entries.get(Self::SIZE_ENTRY) == u64::try_from(size_of::<Self>()).ok()
    }

    /// the cause when the size entry resolved to another size than the mirror; `None` when it fits or is missing
    fn misfit(entries: &GameEntries) -> Option<String> {
        let found = entries.get(Self::SIZE_ENTRY)?;
        (!Self::fits(entries))
            .then(|| format!("{} is {found:#x}, the mirror is {:#x}", Self::SIZE_ENTRY.as_str(), size_of::<Self>()))
    }
}
