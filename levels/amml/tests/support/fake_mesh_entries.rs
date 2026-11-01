use alaydriem_levels::types::{LevelsEntry, LevelsFeature, MeshCalls};
use amml_game::{CameraLayout, EntryId};

use crate::support::FakeResolver;

// the entries of the mesh path with values that fit the mirrors; the code addresses are fakes no test calls
pub struct FakeMeshEntries;

impl FakeMeshEntries {
    // every entry of the markers feature, the screen tessellator and the camera anchor
    pub fn values() -> Vec<(EntryId, u64)> {
        LevelsFeature::entries(LevelsFeature::MARKERS)
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let value = match *id {
                    LevelsEntry::MESH_SIZE => 608,
                    LevelsEntry::TESSELLATOR_TESSELLATING => 0x2A0,
                    LevelsEntry::RESOURCE_LOCATION_FULL_HASH => 0x30,
                    LevelsEntry::IMAGE_BUFFER_DESCRIPTION => 0x18,
                    LevelsEntry::IMAGE_DESCRIPTION_DEPTH => 0x10,
                    LevelsEntry::TEXTURE_GROUP_BASE_OFFSET => 0x18,
                    LevelsEntry::SCREEN_CONTEXT_MESH_CONTEXT => 0x10,
                    LevelsEntry::SCREEN_CONTEXT_CAMERA => 0x18,
                    LevelsEntry::SCREEN_CONTEXT_CONSTANT_BUFFERS => 0x20,
                    LevelsEntry::SCREEN_CONTEXT_CURRENT_SHADER_COLOR => 0x30,
                    _ => 0x1000 + index as u64,
                };
                (*id, value)
            })
            .chain([(LevelsEntry::SCREEN_CONTEXT_TESSELLATOR, 0x58), (EntryId::CAMERA_FORWARD, CameraLayout::FORWARD)])
            .collect()
    }

    pub fn calls() -> MeshCalls {
        MeshCalls::new(&FakeResolver::entries(&Self::values())).unwrap()
    }
}
