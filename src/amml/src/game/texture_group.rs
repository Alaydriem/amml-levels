use std::ffi::c_void;

use amml_game::RawObject;
use amml_msvc::SharedPtr;

use crate::game::{
    ImageBufferLayout, ImageDescriptionLayout, PathViewLayout, ResourceLocationLayout, TexturePtrLayout,
};
use crate::types::MeshCalls;

// std::shared_ptr<mce::TextureGroup> IClientInstance::getTextureGroup() const: this, then the hidden result
type GetTextureGroupFn = unsafe extern "C" fn(*mut c_void, *mut SharedPtr<c_void>) -> *mut SharedPtr<c_void>;
// ResourceLocation::ResourceLocation(Core::PathView): the buffer, then a pointer to the view
type LocationCtorFn =
    unsafe extern "C" fn(*mut ResourceLocationLayout, *const PathViewLayout) -> *mut ResourceLocationLayout;
// cg::ImageBuffer::ImageBuffer(cg::ImageDescription const&)
type ImageBufferCtorFn = unsafe extern "C" fn(*mut ImageBufferLayout, *const ImageDescriptionLayout) -> *mut c_void;
// the blob deleter an ImageBuffer stores at +0
type BlobDeleterFn = unsafe extern "C" fn(*mut u8);
// BedrockTexture& mce::TextureGroup::uploadTexture(ResourceLocation const&, cg::ImageBuffer): the group, the
// location, then the buffer passed by pointer, which the callee frees
type UploadFn = unsafe extern "C" fn(*mut c_void, *const ResourceLocationLayout, *mut ImageBufferLayout) -> *mut c_void;
// mce::TexturePtr mce::TextureGroupBase::getTexture(ResourceLocation const&, bool forceReload, std::optional<uint>,
// cg::TextureSetLayerType): the base subobject, the hidden result, the location, then the three values
type GetFn = unsafe extern "C" fn(
    *mut c_void,
    *mut TexturePtrLayout,
    *const ResourceLocationLayout,
    u8,
    u64,
    u32,
) -> *mut TexturePtrLayout;
// bool mce::TextureGroup::unloadTexture(ResourceLocation const&, bool assertOnFailure)
type UnloadFn = unsafe extern "C" fn(*mut c_void, *const ResourceLocationLayout, u8) -> u8;

/// The client's `mce::TextureGroup`, held through one use of its `shared_ptr`, which the drop gives back. Uploads
/// and looks up textures by resource location. Render thread only
pub struct TextureGroup {
    group: SharedPtr<c_void>,
    calls: MeshCalls,
}

impl TextureGroup {
    /// The texture group of `client`; `None` when the client has none
    ///
    /// # Safety
    /// `client` must be a live `ClientInstance` of the build `calls` was resolved for
    #[allow(unsafe_code)]
    pub unsafe fn of_client(client: *mut c_void, calls: &MeshCalls) -> Option<Self> {
        if client.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees a live client, whose vtable holds the resolved slot
        let function = unsafe { RawObject::slot(client, calls.get_texture_group_slot) }?;
        // SAFETY: the slot holds `getTextureGroup` with the signature of `GetTextureGroupFn`
        let get: GetTextureGroupFn = unsafe { std::mem::transmute(function) };
        // SAFETY: an all-zero shared_ptr is the empty pointer
        let mut group: SharedPtr<c_void> = unsafe { std::mem::zeroed() };
        // SAFETY: the call writes a shared_ptr, with one use for the mod, through the hidden result pointer
        unsafe { get(client, &mut group) };
        if group.get().is_none() {
            // SAFETY: an empty pointer or one holding the use the call gave
            unsafe { group.release() };
            return None;
        }
        Some(Self { group, calls: *calls })
    }

    fn address(&self) -> *mut c_void {
        self.group.get().map_or(std::ptr::null_mut(), |group| std::ptr::from_ref(group).cast_mut())
    }

    /// A resource location for `path`, built by the game constructor; its path string is on the game heap
    #[allow(unsafe_code)]
    pub fn location(&self, path: &str) -> Box<ResourceLocationLayout> {
        let mut location = Box::new(ResourceLocationLayout::zeroed());
        let view = PathViewLayout::new(path);
        // SAFETY: the entry is the address of `ResourceLocation(Core::PathView)` with the signature of
        // `LocationCtorFn`
        let ctor: LocationCtorFn = unsafe { std::mem::transmute(self.calls.resource_location_ctor) };
        // SAFETY: a zeroed buffer of the class size and more, and a view of `path`, which outlives the call; the game
        // copies it
        unsafe { ctor(&mut *location, &view) };
        location
    }

    /// Uploads a square RGBA8 image of `side` pixels under `location`; false when the game made no buffer of the
    /// size of `pixels`
    #[allow(unsafe_code)]
    pub fn upload(&self, location: &ResourceLocationLayout, side: u32, pixels: &[u8]) -> bool {
        let description = ImageDescriptionLayout::square(side);
        let mut buffer = ImageBufferLayout::zeroed();
        // SAFETY: the entry is the address of `ImageBuffer(ImageDescription const&)` with the signature of
        // `ImageBufferCtorFn`
        let ctor: ImageBufferCtorFn = unsafe { std::mem::transmute(self.calls.image_buffer_ctor) };
        // SAFETY: a zeroed buffer of the class size and more, and a description, that live for the call
        unsafe { ctor(&mut buffer, &description) };
        if buffer.data.is_null() {
            return false;
        }
        if buffer.size != pixels.len() {
            if buffer.deleter != 0 {
                // SAFETY: a non-zero deleter is the one the constructor stored for this blob
                let delete: BlobDeleterFn = unsafe { std::mem::transmute(buffer.deleter) };
                // SAFETY: the blob is the constructor's and is not used again
                unsafe { delete(buffer.data) };
            }
            return false;
        }
        // SAFETY: the size check shows the blob holds exactly `pixels.len()` bytes
        unsafe { std::ptr::copy_nonoverlapping(pixels.as_ptr(), buffer.data, pixels.len()) };
        // SAFETY: the entry is the address of `TextureGroup::uploadTexture` with the signature of `UploadFn`
        let upload: UploadFn = unsafe { std::mem::transmute(self.calls.texture_upload) };
        // SAFETY: the live group, a built location and a built buffer, which the callee takes and frees
        unsafe { upload(self.address(), location, &mut buffer) };
        true
    }

    /// The texture under `location`, holding one use of each pointer for the caller; empty when the group has none
    #[allow(unsafe_code)]
    pub fn texture(&self, location: &ResourceLocationLayout) -> TexturePtrLayout {
        let mut texture = TexturePtrLayout::EMPTY;
        // SAFETY: the entry is the address of `TextureGroupBase::getTexture` with the signature of `GetFn`
        let get: GetFn = unsafe { std::mem::transmute(self.calls.texture_get) };
        // `getTexture` is a member of the `TextureGroupBase` subobject, `TextureGroup.baseOffset` into the group
        let base = self.address().cast::<u8>().wrapping_add(self.calls.texture_group_base as usize).cast::<c_void>();
        // SAFETY: the base subobject of the live group, an empty result to fill and a built location; no reload, no
        // mip count, the default layer type
        unsafe { get(base, &mut texture, location, 0, 0, 0) };
        texture
    }

    /// Unloads the texture under `location` from the group
    #[allow(unsafe_code)]
    pub fn unload(&self, location: &ResourceLocationLayout) {
        // SAFETY: the entry is the address of `TextureGroup::unloadTexture` with the signature of `UnloadFn`
        let unload: UnloadFn = unsafe { std::mem::transmute(self.calls.texture_unload) };
        // SAFETY: the live group and a built location
        unsafe { unload(self.address(), location, 0) };
    }
}

impl Drop for TextureGroup {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: the pointer holds the use `getTextureGroup` gave, given back once
        unsafe { self.group.release() };
    }
}
