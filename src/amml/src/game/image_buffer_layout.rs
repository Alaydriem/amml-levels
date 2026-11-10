use std::mem::offset_of;

/// Layout mirror of the game's `cg::ImageBuffer` (0x30 bytes): the blob deleter, the pixel data and its size, then
/// the description. `ImageBuffer(ImageDescription const&)` builds it with a blob of the image size;
/// `TextureGroup::uploadTexture` takes it by value and frees the blob. The mirror ends with room past the class, so
/// a member the game adds at the end is written into the mirror and not past it; `ImageBuffer.mDescription` checks
/// the members the mod reads
#[repr(C)]
pub struct ImageBufferLayout {
    pub deleter: usize,    // 0x00 blob deleter
    pub data: *mut u8,     // 0x08 blob data
    pub size: usize,       // 0x10 blob size
    description: [u64; 3], // 0x18 ImageDescription and padding
    _headroom: [u64; 8],   // 0x30 room past the class end
}

impl ImageBufferLayout {
    // where the description copy sits: the anchor `ImageBuffer.mDescription` must give
    pub const DESCRIPTION: u64 = offset_of!(ImageBufferLayout, description) as u64;

    // an all-zero buffer for the game constructor
    pub fn zeroed() -> Self {
        Self { deleter: 0, data: std::ptr::null_mut(), size: 0, description: [0; 3], _headroom: [0; 8] }
    }
}

const _: () = assert!(offset_of!(ImageBufferLayout, size) == 0x10);
const _: () = assert!(offset_of!(ImageBufferLayout, _headroom) == 0x30);
