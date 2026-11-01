use std::ffi::c_void;

use amml_game::GameHeap;

use crate::game::{ResourceLocationLayout, TextureGroup, TexturePtrLayout, TextureVariant};
use crate::types::MeshCalls;

/// A one-pixel opaque white texture the mod uploads into the client's texture group, so a world mesh drawn with
/// the name-tag material shows its vertex colours. A render-device object: made on the render thread, dropped there
/// with the meshes. The drop gives back the texture's uses, unloads it from the group and frees the location's path
pub struct WhiteTexture {
    group: TextureGroup,
    location: Box<ResourceLocationLayout>,
    texture: TexturePtrLayout,
}

impl WhiteTexture {
    // any UV inside the one pixel
    pub const UV: [f32; 2] = [0.5, 0.5];
    const PIXELS: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];

    /// Uploads the texture under `path` and takes it back from the group; `None` when the client has no group, the
    /// upload fails or the group gives no texture back. One path only, so the group's map gains at most one node
    ///
    /// # Safety
    /// `client` must be a live `ClientInstance` of the build `calls` was resolved for, and the call must be on the
    /// render thread
    #[allow(unsafe_code)]
    pub unsafe fn load(client: *mut c_void, calls: &MeshCalls, path: &str) -> Option<Self> {
        // SAFETY: forwarded to the caller
        let group = unsafe { TextureGroup::of_client(client, calls) }?;
        let location = group.location(path);
        let mut white = Self { group, location, texture: TexturePtrLayout::EMPTY };
        if !white.group.upload(&white.location, 1, &Self::PIXELS) {
            return None;
        }
        white.texture = white.group.texture(&white.location);
        if white.texture.is_empty() {
            return None;
        }
        Some(white)
    }

    // the texture as `Mesh::renderMesh` takes it; the variant borrows the uses this value holds
    pub fn variant(&self) -> TextureVariant {
        TextureVariant::new(TexturePtrLayout::VARIANT_INDEX, self.texture)
    }
}

impl Drop for WhiteTexture {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: the texture holds the uses `getTexture` gave, or is empty; it is released once
        unsafe { self.texture.release() };
        self.group.unload(&self.location);
        if let Some(heap) = GameHeap::msvc() {
            // SAFETY: the game constructor built the path string on the game heap, and the location is not used again
            unsafe { self.location.path.release(&heap) };
        }
    }
}
