use std::mem::offset_of;

use amml_game::{CameraLayout, EntryId, GameEntries};

use crate::errors::LayoutFault;
use crate::game::{
    ImageBufferLayout, ImageDescriptionLayout, MeshContextLayout, MeshLayout2651, MeshLayoutKind,
    ResourceLocationLayout, TessellatorLayout,
};
use crate::types::LevelsEntry;

/// The resolved game code and offsets of the world mesh path: build a mesh on the screen's tessellator, push a
/// world matrix, set the actor constants, upload the white texture and draw. Code addresses and byte offsets, as
/// the entries give them. A test fills it from fake entries
#[derive(Clone, Copy, Debug)]
pub struct MeshCalls {
    pub tessellator_begin: usize,
    pub tessellator_end: usize,
    pub tessellator_clear: usize,
    pub mesh_dtor: usize,
    pub mesh_is_valid: usize,
    pub mesh_render: usize,
    pub set_entity_constants: usize,
    pub matrix_stack_push: usize,
    pub matrix_stack_ref_dtor: usize,
    /// offset of `entityConstantBuffer` in `GlobalConstantBuffers`
    pub entity_constant_buffer: u64,
    /// offset of the `Tessellator&` in `ScreenContext`
    pub screen_tessellator: u64,
    /// offset of `mViews` in `FrameRenderObject`
    pub frame_views: u64,
    /// vtable slot of `ClientInstance::getTextureGroup`
    pub get_texture_group_slot: u64,
    pub texture_get: usize,
    pub texture_upload: usize,
    pub texture_unload: usize,
    pub resource_location_ctor: usize,
    pub image_buffer_ctor: usize,
    /// offset of the `TextureGroupBase` subobject in `TextureGroup`, the `this` of `getTexture`
    pub texture_group_base: u64,
    /// offset of the `MeshContext` base in `ScreenContext`, the context `renderMesh` takes
    pub mesh_context: u64,
}

impl MeshCalls {
    /// The calls of the build `entries` was resolved for. An error names the first entry that is missing, or the
    /// mirror anchor that does not fit (see `misfit`)
    pub fn new(entries: &GameEntries) -> Result<Self, LayoutFault> {
        if let Some(fault) = Self::misfit(entries) {
            return Err(fault);
        }
        let get = |id: EntryId| entries.get(id).ok_or(LayoutFault::Missing { entry: id.as_str() });
        let address = |id: EntryId| get(id).map(|value| value as usize);
        Ok(Self {
            tessellator_begin: address(LevelsEntry::TESSELLATOR_BEGIN)?,
            tessellator_end: address(LevelsEntry::TESSELLATOR_END)?,
            tessellator_clear: address(LevelsEntry::TESSELLATOR_CLEAR)?,
            mesh_dtor: address(LevelsEntry::MESH_DTOR)?,
            mesh_is_valid: address(LevelsEntry::MESH_IS_VALID)?,
            mesh_render: address(LevelsEntry::MESH_RENDER)?,
            set_entity_constants: address(LevelsEntry::SET_ENTITY_CONSTANTS)?,
            matrix_stack_push: address(LevelsEntry::MATRIX_STACK_PUSH)?,
            matrix_stack_ref_dtor: address(LevelsEntry::MATRIX_STACK_REF_DTOR)?,
            entity_constant_buffer: get(LevelsEntry::ENTITY_CONSTANT_BUFFER)?,
            screen_tessellator: get(LevelsEntry::SCREEN_CONTEXT_TESSELLATOR)?,
            frame_views: get(LevelsEntry::FRAME_VIEWS)?,
            get_texture_group_slot: get(LevelsEntry::GET_TEXTURE_GROUP)?,
            texture_get: address(LevelsEntry::TEXTURE_GROUP_GET_TEXTURE)?,
            texture_upload: address(LevelsEntry::TEXTURE_GROUP_UPLOAD_TEXTURE)?,
            texture_unload: address(LevelsEntry::TEXTURE_GROUP_UNLOAD_TEXTURE)?,
            resource_location_ctor: address(LevelsEntry::RESOURCE_LOCATION_CTOR)?,
            image_buffer_ctor: address(LevelsEntry::IMAGE_BUFFER_CTOR)?,
            texture_group_base: get(LevelsEntry::TEXTURE_GROUP_BASE_OFFSET)?,
            mesh_context: get(LevelsEntry::SCREEN_CONTEXT_MESH_CONTEXT)?,
        })
    }

    /// The first mirror anchor that resolved to another value than the mirror holds; `None` when each fits or is
    /// missing (a missing entry is named by the feature list). The anchors: the offsets of `Tessellator.mTessellating`
    /// and `Camera.mForward`; the last member of `ResourceLocation`, of `cg::ImageDescription` and the description in
    /// `cg::ImageBuffer`, the three classes the mod builds in its own buffers; the camera, constant buffer and shader
    /// colour references of the `MeshContext` base, relative to `ScreenContext.meshContext`; and `Mesh.size`, which
    /// must be the size of one of the mesh mirrors
    pub fn misfit(entries: &GameEntries) -> Option<LayoutFault> {
        let anchor = |id: EntryId, expected: u64| {
            let found = entries.get(id)?;
            (found != expected).then_some(LayoutFault::Misfit { entry: id.as_str(), expected, found })
        };
        let in_mesh_context = |id: EntryId, field: usize| {
            let base = entries.get(LevelsEntry::SCREEN_CONTEXT_MESH_CONTEXT)?;
            anchor(id, base + field as u64)
        };
        let tessellating = offset_of!(TessellatorLayout, tessellating) as u64;
        anchor(LevelsEntry::TESSELLATOR_TESSELLATING, tessellating)
            .or_else(|| anchor(EntryId::CAMERA_FORWARD, CameraLayout::FORWARD))
            .or_else(|| anchor(LevelsEntry::RESOURCE_LOCATION_FULL_HASH, ResourceLocationLayout::FULL_HASH))
            .or_else(|| anchor(LevelsEntry::IMAGE_BUFFER_DESCRIPTION, ImageBufferLayout::DESCRIPTION))
            .or_else(|| anchor(LevelsEntry::IMAGE_DESCRIPTION_DEPTH, ImageDescriptionLayout::DEPTH))
            .or_else(|| in_mesh_context(LevelsEntry::SCREEN_CONTEXT_CAMERA, offset_of!(MeshContextLayout, camera)))
            .or_else(|| {
                in_mesh_context(
                    LevelsEntry::SCREEN_CONTEXT_CONSTANT_BUFFERS,
                    offset_of!(MeshContextLayout, constant_buffers),
                )
            })
            .or_else(|| {
                in_mesh_context(
                    LevelsEntry::SCREEN_CONTEXT_CURRENT_SHADER_COLOR,
                    offset_of!(MeshContextLayout, current_shader_color),
                )
            })
            .or_else(|| {
                let found = entries.get(LevelsEntry::MESH_SIZE)?;
                MeshLayoutKind::of(entries).is_none().then_some(LayoutFault::Misfit {
                    entry: LevelsEntry::MESH_SIZE.as_str(),
                    expected: MeshLayout2651::SIZE,
                    found,
                })
            })
    }
}
