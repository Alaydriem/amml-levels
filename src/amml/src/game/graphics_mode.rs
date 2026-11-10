use std::ffi::c_void;

use amml_game::{EntryId, GameEntries, RawObject};

use crate::types::LevelsEntry;

/// The graphics mode setting, through `ClientInstance::getOptions` and the option registry getter that compares
/// `GRAPHICS_MODE` with Deferred (Vibrant Visuals)
pub struct GraphicsMode;

impl GraphicsMode {
    /// whether the graphics mode setting is Vibrant Visuals. `None` when an entry is off or the options pointer is
    /// null
    ///
    /// # Safety
    /// `client` must be a live ClientInstance of the build the entries describe, read while a world is active
    #[allow(unsafe_code)]
    pub unsafe fn is_vibrant(client: *mut c_void, entries: &GameEntries) -> Option<bool> {
        if client.is_null() {
            return None;
        }
        let options_slot = entries.get(EntryId::CLIENT_INSTANCE_GET_OPTIONS)?;
        let deferred_slot = entries.get(LevelsEntry::GRAPHICS_MODE_DEFERRED)?;
        // SAFETY: `client` is a live ClientInstance, whose first qword is its vtable; the entry is a slot of it
        let get_options = unsafe { RawObject::slot(client, options_slot) }?;
        // SAFETY: the slot holds `IOptionRegistry& ClientInstance::getOptions()`, a reference returned as a pointer
        let options = unsafe { RawObject::call_pointer(get_options, client) }?;
        // SAFETY: `options` is the live IOptionRegistry; the entry is a slot of its vtable
        let is_deferred = unsafe { RawObject::slot(options, deferred_slot) }?;
        // SAFETY: the slot holds `bool IOptionRegistry::isGraphicsModeDeferred() const`
        Some(unsafe { RawObject::call_bool(is_deferred, options) })
    }
}
