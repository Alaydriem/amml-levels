/// Layout mirror of the game's `cg::ImageDescription` (26.60, 20 bytes): the size, the pixel format and the colour
/// flags of an image
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageDescriptionLayout {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub color_space: u8,
    pub image_type: u8,
    pub alpha_usage: u8,
    pub reserved: u8,
    pub depth: u32,
}

impl ImageDescriptionLayout {
    // the last member: the anchor `ImageDescription.depth` must give
    pub const DEPTH: u64 = std::mem::offset_of!(ImageDescriptionLayout, depth) as u64;
    // `mce::TextureFormat::R8G8B8A8_UNORM`
    pub const RGBA8: u32 = 0x1C;
    // `cg::ColorSpace::sRGB`
    const SRGB: u8 = 1;

    // a square RGBA8 sRGB image, as `addEmptyTexture` builds by default
    pub fn square(side: u32) -> Self {
        Self {
            width: side,
            height: side,
            format: Self::RGBA8,
            color_space: Self::SRGB,
            image_type: 0,
            alpha_usage: 0,
            reserved: 0,
            depth: 1,
        }
    }
}

const _: () = assert!(size_of::<ImageDescriptionLayout>() == 20);
