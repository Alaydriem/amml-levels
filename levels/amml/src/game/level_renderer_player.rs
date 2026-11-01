use std::ffi::c_void;

use amml_game::GameEntries;

use crate::types::LevelsEntry;

// the reads of the `LevelRendererPlayer` class the hook needs
pub struct LevelRendererPlayer;

impl LevelRendererPlayer {
    // the address of `extractNameTags`: the code pointer in its slot of the class vtable. `None` when an entry is
    // off or the slot is null. Called once, when the hook is installed
    #[allow(unsafe_code)]
    pub fn extract_name_tags_target(entries: &GameEntries) -> Option<u64> {
        let vtable = usize::try_from(entries.get(LevelsEntry::LEVEL_RENDERER_PLAYER_VTABLE)?).ok()?;
        let slot = usize::try_from(entries.get(LevelsEntry::EXTRACT_NAME_TAGS)?).ok()?;
        if vtable == 0 {
            return None;
        }
        let at = (vtable as *const *const c_void).wrapping_add(slot);
        // SAFETY: the vtable entry is the address of the class's primary vtable in the game image, which holds at
        // least `slot + 1` code pointers
        let function = unsafe { at.read() };
        (!function.is_null()).then_some(function as u64)
    }
}
