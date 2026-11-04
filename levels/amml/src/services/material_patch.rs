use amml_game::{EntryId, GameEntries};

use crate::game::WireframeMaterial;
use crate::types::{LevelsEntry, PatchedBytes};

// makes the wireframe material alpha blend (SourceAlpha, OneMinusSrcAlpha) and stop writing depth, and puts the
// game's bytes back on restore
pub struct MaterialPatch {
    patched: Option<PatchedBytes>,
}

impl Default for MaterialPatch {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialPatch {
    // the mce::BlendTarget values the draw-state builder maps to the alpha blend mode
    pub const SOURCE_ALPHA: u8 = 6;
    pub const ONE_MINUS_SOURCE_ALPHA: u8 = 8;

    pub fn new() -> Self {
        Self { patched: None }
    }

    pub fn is_applied(&self) -> bool {
        self.patched.is_some()
    }

    /// writes blend on with the alpha blend factors and depth write off. When the material address differs from the kept one, the kept bytes are
    /// forgotten and nothing is written to the old address (it may be freed). Does nothing when an entry is off
    ///
    /// # Safety
    /// `material` must have been read from the live level renderer in the current render call
    #[allow(unsafe_code)]
    pub unsafe fn apply(&mut self, material: &WireframeMaterial, entries: &GameEntries) {
        let Some(offsets) = Self::offsets(entries) else { return };
        let address = material.render_material();
        if self.patched.is_some_and(|kept| kept.material != address) {
            self.patched = None;
        }
        let [blend, source, destination, depth] = offsets.map(|at| address.as_ptr().wrapping_add(at));
        if self.patched.is_none() {
            // SAFETY: `address` was read from the live chain in this render call, so it is a live RenderMaterial
            // and the entries are its field offsets
            let kept = unsafe {
                PatchedBytes {
                    material: address,
                    blend: blend.read(),
                    source: source.read(),
                    destination: destination.read(),
                    depth_write: depth.read(),
                }
            };
            self.patched = Some(kept);
        }
        let wanted =
            [(blend, 1), (source, Self::SOURCE_ALPHA), (destination, Self::ONE_MINUS_SOURCE_ALPHA), (depth, 0)];
        for (field, value) in wanted {
            // SAFETY: the same live RenderMaterial as above
            unsafe {
                if field.read() != value {
                    field.write(value);
                }
            }
        }
    }

    /// writes the kept bytes back once when `current`, read from the live renderer in this call, is the kept
    /// material; otherwise writes nothing. The kept bytes are forgotten either way
    ///
    /// # Safety
    /// `current` must have been read from the live level renderer in the current render call
    #[allow(unsafe_code)]
    pub unsafe fn restore_if_current(&mut self, current: Option<&WireframeMaterial>, entries: &GameEntries) {
        let Some(kept) = self.patched.take() else { return };
        let Some(current) = current.filter(|current| current.render_material() == kept.material) else { return };
        let Some(offsets) = Self::offsets(entries) else { return };
        let address = current.render_material().as_ptr();
        let values = [kept.blend, kept.source, kept.destination, kept.depth_write];
        for (at, value) in offsets.into_iter().zip(values) {
            // SAFETY: `current` was just read from the live chain and has the kept address, so the RenderMaterial
            // is live
            unsafe { address.wrapping_add(at).write(value) };
        }
    }

    // drops the kept bytes without writing: the material may be gone
    pub fn forget(&mut self) {
        self.patched = None;
    }

    // blend enable, blend source, blend destination, depth write
    fn offsets(entries: &GameEntries) -> Option<[usize; 4]> {
        Some([
            Self::offset(entries, LevelsEntry::MATERIAL_ENABLE_BLEND)?,
            Self::offset(entries, LevelsEntry::MATERIAL_BLEND_SOURCE)?,
            Self::offset(entries, LevelsEntry::MATERIAL_BLEND_DESTINATION)?,
            Self::offset(entries, LevelsEntry::MATERIAL_DEPTH_WRITE)?,
        ])
    }

    fn offset(entries: &GameEntries, id: EntryId) -> Option<usize> {
        usize::try_from(entries.get(id)?).ok()
    }
}
