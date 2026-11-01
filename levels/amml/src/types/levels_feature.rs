use amml_game::{EntryId, GameEntries};

use crate::game::{FontHandle, GameMirror};
use crate::types::{LevelsEntry, MeshCalls};

// the optional parts of the mod; a feature is off when one of its entries did not resolve or a mirror it reads
// does not fit the build
pub struct LevelsFeature;

impl LevelsFeature {
    // label widths from the game font; without it a fixed width table is used
    pub const FONT: &'static str = "font";
    // the world meshes: chunk borders and spawn spheres
    pub const MARKERS: &'static str = "markers";

    pub const ALL: [&'static str; 2] = [Self::FONT, Self::MARKERS];

    pub fn entries(feature: &str) -> &'static [EntryId] {
        match feature {
            Self::FONT => &[
                LevelsEntry::GET_FONT_HANDLE,
                LevelsEntry::FONT_HANDLE_DTOR,
                LevelsEntry::FONT_HANDLE_SIZE,
                LevelsEntry::FONT_GET_LINE_LENGTH,
            ],
            Self::MARKERS => &[
                LevelsEntry::RENDER_LEVEL,
                LevelsEntry::FRAME_VIEWS,
                LevelsEntry::TESSELLATOR_BEGIN,
                LevelsEntry::TESSELLATOR_END,
                LevelsEntry::TESSELLATOR_CLEAR,
                LevelsEntry::TESSELLATOR_TESSELLATING,
                LevelsEntry::MESH_DTOR,
                LevelsEntry::MESH_IS_VALID,
                LevelsEntry::MESH_RENDER,
                LevelsEntry::MESH_SIZE,
                LevelsEntry::MATRIX_STACK_PUSH,
                LevelsEntry::MATRIX_STACK_REF_DTOR,
                LevelsEntry::SET_ENTITY_CONSTANTS,
                LevelsEntry::ENTITY_CONSTANT_BUFFER,
                LevelsEntry::GET_TEXTURE_GROUP,
                LevelsEntry::TEXTURE_GROUP_GET_TEXTURE,
                LevelsEntry::TEXTURE_GROUP_UPLOAD_TEXTURE,
                LevelsEntry::TEXTURE_GROUP_UNLOAD_TEXTURE,
                LevelsEntry::RESOURCE_LOCATION_CTOR,
                LevelsEntry::IMAGE_BUFFER_CTOR,
                LevelsEntry::RESOURCE_LOCATION_FULL_HASH,
                LevelsEntry::IMAGE_BUFFER_DESCRIPTION,
                LevelsEntry::IMAGE_DESCRIPTION_DEPTH,
                LevelsEntry::TEXTURE_GROUP_BASE_OFFSET,
                LevelsEntry::SCREEN_CONTEXT_MESH_CONTEXT,
                LevelsEntry::SCREEN_CONTEXT_CAMERA,
                LevelsEntry::SCREEN_CONTEXT_CONSTANT_BUFFERS,
                LevelsEntry::SCREEN_CONTEXT_CURRENT_SHADER_COLOR,
            ],
            _ => &[],
        }
    }

    // whether every entry of the feature resolved and its mirrors fit
    pub fn is_on(entries: &GameEntries, feature: &str) -> bool {
        Self::entries(feature).iter().all(|id| entries.get(*id).is_some()) && Self::misfit(entries, feature).is_none()
    }

    // the cause when a mirror the feature reads does not fit the build
    pub fn misfit(entries: &GameEntries, feature: &str) -> Option<String> {
        match feature {
            Self::FONT => FontHandle::misfit(entries),
            Self::MARKERS => MeshCalls::misfit(entries).map(|fault| fault.to_string()),
            _ => None,
        }
    }
}
