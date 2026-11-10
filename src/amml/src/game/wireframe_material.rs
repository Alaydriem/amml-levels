use std::ffi::c_void;
use std::ptr::NonNull;

use amml_game::GameEntries;

use crate::types::LevelsEntry;

// the level renderer's `wireframe` material: the `mce::MaterialPtr` the game keeps in its player renderer and the
// `RenderMaterial` behind it
pub struct WireframeMaterial {
    material_ptr: *const c_void,
    render_material: NonNull<u8>,
}

impl WireframeMaterial {
    /// follows LevelRenderer -> LevelRendererPlayer -> inline MaterialPtr -> RenderMaterialInfo -> RenderMaterial.
    /// `None` when an entry is off or a hop is null
    ///
    /// # Safety
    /// `level_renderer` must be null or a live LevelRenderer of the build the entries describe
    #[allow(unsafe_code)]
    pub unsafe fn from_level_renderer(
        level_renderer: *const c_void,
        entries: &GameEntries,
    ) -> Option<WireframeMaterial> {
        if level_renderer.is_null() {
            return None;
        }
        let player_at = usize::try_from(entries.get(LevelsEntry::LEVEL_RENDERER_PLAYER)?).ok()?;
        let material_at = usize::try_from(entries.get(LevelsEntry::WIREFRAME_MATERIAL)?).ok()?;
        let inner_at = usize::try_from(entries.get(LevelsEntry::MATERIAL_INFO_MATERIAL)?).ok()?;
        // SAFETY: `level_renderer` is a live LevelRenderer; the entry is the offset of its player pointer field
        let player = unsafe { level_renderer.cast::<u8>().add(player_at).cast::<*const u8>().read_unaligned() };
        if player.is_null() {
            return None;
        }
        // the MaterialPtr is stored inline in the player renderer; its first word is the shared object pointer
        let material_ptr = player.wrapping_add(material_at);
        // SAFETY: `player` is the non-null LevelRendererPlayer and the entry the offset of its inline MaterialPtr
        let info = unsafe { material_ptr.cast::<*const u8>().read_unaligned() };
        if info.is_null() {
            return None;
        }
        // SAFETY: `info` is the RenderMaterialInfo the MaterialPtr points at; the entry is its material field offset
        let render_material = unsafe { info.add(inner_at).cast::<*mut u8>().read_unaligned() };
        Some(WireframeMaterial { material_ptr: material_ptr.cast(), render_material: NonNull::new(render_material)? })
    }

    // the `MaterialPtr const*` that `renderMesh` takes
    pub fn material_ptr(&self) -> *const c_void {
        self.material_ptr
    }

    pub fn render_material(&self) -> NonNull<u8> {
        self.render_material
    }
}
