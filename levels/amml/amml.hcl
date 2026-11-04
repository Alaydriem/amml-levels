namespace   = "alaydriem"
name        = "levels"
entry       = "alaydriem_levels.dll"
api_version = 2
entries = [
  # required: the overlay refuses to enable without them
  "ClientInstance.update",
  "ClientInstance.getLocalPlayer",
  "ClientInstance.isWorldActive",
  "ClientInstance.isShowingMenu",
  "ClientInstance.getOptions",
  "IOptionRegistry.getHideHud",
  "Actor.stateVectorComponent",
  "Actor.actorRotationComponent",
  "StateVectorComponent.mPos",
  "ActorRotationComponent.mRot",
  "Actor.getDimension",
  "Dimension.getDimensionId",
  "Actor.getDimensionBlockSource",
  "BlockSource.getChunk",
  "LevelChunk.mSubChunks",
  "LevelChunk.getBlock",
  "BlockSource.getBrightnessPair",
  "Block.canProvideFullSupport",
  "BlockType.getCollisionShape",
  "Camera.mForward",
  "ViewRenderObject.mViewData",
  "ViewRenderData.mCameraPos",
  "ScreenContext.tessellator",
  "BaseActorRenderer.ctor",
  "BaseActorRenderer.size",
  "IOptionRegistry.isGraphicsModeDeferred",
  "LevelRenderer.renderLevel",
  "FrameRenderObject.mViews",
  "Tessellator.begin",
  "Tessellator.end",
  "Tessellator.clear",
  "Tessellator.mTessellating",
  "Mesh.dtor",
  "Mesh.isValid",
  "Mesh.renderMesh",
  "Mesh.size",
  "MatrixStack.push",
  "MatrixStackRef.dtor",
  "ActorShaderManager.setEntityConstants",
  "GlobalConstantBuffers.entityConstantBuffer",
  "ClientInstance.getTextureGroup",
  "TextureGroup.getTexture",
  "TextureGroup.uploadTexture",
  "TextureGroup.unloadTexture",
  "ResourceLocation.ctor",
  "ImageBuffer.ctorFromDescription",
  "ResourceLocation.mFullHash",
  "ImageBuffer.mDescription",
  "ImageDescription.depth",
  "TextureGroup.baseOffset",
  "ScreenContext.meshContext",
  "ScreenContext.camera",
  "ScreenContext.constantBuffers",
  "ScreenContext.currentShaderColor",
  "LevelRenderer.mLevelRendererPlayer",
  "LevelRendererCamera.mWireframeMaterial",
  "RenderMaterialInfo.mMaterial",
  "RenderMaterial.mEnableBlend",
  "RenderMaterial.mDepthWriteMask",
  "RenderMaterial.mBlendSource",
  "RenderMaterial.mBlendDestination",
]

listing {
  title       = "Levels"
  summary     = "Light levels, chunk borders, and spawn sphere markers."
  description = <<-EOT
    View levels and place markers in your world to see light levels, chunk borders, spawn spheres, and more.
  EOT
  authors     = ["Charles R. Portwood II"]
  license     = "BSD-3-Clause"
  source      = "https://github.com/alaydriem/amml-levels"
  homepage    = "https://github.com/alaydriem/amml-levels#readme"
  tags        = ["light-level", "spawning", "chunk-borders", "overlay", "client"]
  icon        = "icon.png"
}
