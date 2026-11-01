use amml_game::EntryId;

// the entries of the name-tag draw path, the font reads and the world mesh path; the shared ones are in amml-game
pub struct LevelsEntry;

impl LevelsEntry {
    pub const LEVEL_RENDERER_PLAYER_VTABLE: EntryId = EntryId::new("LevelRendererPlayer.vtable");
    pub const EXTRACT_NAME_TAGS: EntryId = EntryId::new("LevelRendererPlayer.extractNameTags");
    pub const NAME_TAG_SIZE: EntryId = EntryId::new("NameTagRenderObject.size");
    pub const SCREEN_CONTEXT_TESSELLATOR: EntryId = EntryId::new("ScreenContext.tessellator");
    pub const BASE_ACTOR_RENDERER_CTOR: EntryId = EntryId::new("BaseActorRenderer.ctor");
    pub const BASE_ACTOR_RENDERER_SIZE: EntryId = EntryId::new("BaseActorRenderer.size");
    pub const EXTRACT_RENDER_TEXT_OBJECT: EntryId = EntryId::new("BaseActorRenderer._extractRenderTextObject");
    pub const GET_FONT_HANDLE: EntryId = EntryId::new("ClientInstance.getFontHandle");
    pub const FONT_HANDLE_DTOR: EntryId = EntryId::new("FontHandle.dtor");
    pub const FONT_HANDLE_SIZE: EntryId = EntryId::new("FontHandle.size");
    pub const FONT_GET_LINE_LENGTH: EntryId = EntryId::new("Font.getLineLength");
    pub const RENDER_LEVEL: EntryId = EntryId::new("LevelRenderer.renderLevel");
    pub const FRAME_VIEWS: EntryId = EntryId::new("FrameRenderObject.mViews");
    pub const TESSELLATOR_BEGIN: EntryId = EntryId::new("Tessellator.begin");
    pub const TESSELLATOR_END: EntryId = EntryId::new("Tessellator.end");
    pub const TESSELLATOR_CLEAR: EntryId = EntryId::new("Tessellator.clear");
    pub const TESSELLATOR_TESSELLATING: EntryId = EntryId::new("Tessellator.mTessellating");
    pub const MESH_DTOR: EntryId = EntryId::new("Mesh.dtor");
    pub const MESH_IS_VALID: EntryId = EntryId::new("Mesh.isValid");
    pub const MESH_RENDER: EntryId = EntryId::new("Mesh.renderMesh");
    pub const MESH_SIZE: EntryId = EntryId::new("Mesh.size");
    pub const MATRIX_STACK_PUSH: EntryId = EntryId::new("MatrixStack.push");
    pub const MATRIX_STACK_REF_DTOR: EntryId = EntryId::new("MatrixStackRef.dtor");
    pub const SET_ENTITY_CONSTANTS: EntryId = EntryId::new("ActorShaderManager.setEntityConstants");
    pub const ENTITY_CONSTANT_BUFFER: EntryId = EntryId::new("GlobalConstantBuffers.entityConstantBuffer");
    pub const GET_TEXTURE_GROUP: EntryId = EntryId::new("ClientInstance.getTextureGroup");
    pub const TEXTURE_GROUP_GET_TEXTURE: EntryId = EntryId::new("TextureGroup.getTexture");
    pub const TEXTURE_GROUP_UPLOAD_TEXTURE: EntryId = EntryId::new("TextureGroup.uploadTexture");
    pub const TEXTURE_GROUP_UNLOAD_TEXTURE: EntryId = EntryId::new("TextureGroup.unloadTexture");
    pub const RESOURCE_LOCATION_CTOR: EntryId = EntryId::new("ResourceLocation.ctor");
    pub const IMAGE_BUFFER_CTOR: EntryId = EntryId::new("ImageBuffer.ctorFromDescription");
    pub const RESOURCE_LOCATION_FULL_HASH: EntryId = EntryId::new("ResourceLocation.mFullHash");
    pub const IMAGE_BUFFER_DESCRIPTION: EntryId = EntryId::new("ImageBuffer.mDescription");
    pub const IMAGE_DESCRIPTION_DEPTH: EntryId = EntryId::new("ImageDescription.depth");
    pub const TEXTURE_GROUP_BASE_OFFSET: EntryId = EntryId::new("TextureGroup.baseOffset");
    pub const SCREEN_CONTEXT_MESH_CONTEXT: EntryId = EntryId::new("ScreenContext.meshContext");
    pub const SCREEN_CONTEXT_CAMERA: EntryId = EntryId::new("ScreenContext.camera");
    pub const SCREEN_CONTEXT_CONSTANT_BUFFERS: EntryId = EntryId::new("ScreenContext.constantBuffers");
    pub const SCREEN_CONTEXT_CURRENT_SHADER_COLOR: EntryId = EntryId::new("ScreenContext.currentShaderColor");
}
