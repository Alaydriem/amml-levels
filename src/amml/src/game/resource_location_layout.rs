use std::mem::offset_of;

use amml_msvc::MsvcString;

/// Layout mirror of the game's `ResourceLocation` (26.60, 0x38 bytes): the file system, the path and two hashes.
/// The game constructor builds it in place from zeroed bytes; the path string is on the game heap. The mirror ends
/// with room past the class, so a member the game adds at the end is written into the mirror and not past it;
/// `ResourceLocation.mFullHash` checks the members before it
#[repr(C)]
pub struct ResourceLocationLayout {
    pub file_system: u32, // 0x00 mFileSystem
    pub path: MsvcString, // 0x08 mPath
    pub path_hash: u64,   // 0x28 mPathHash
    pub full_hash: u64,   // 0x30 mFullHash
    _headroom: [u64; 8],  // 0x38 room past the class end
}

impl ResourceLocationLayout {
    // the last member: the anchor `ResourceLocation.mFullHash` must give
    pub const FULL_HASH: u64 = offset_of!(ResourceLocationLayout, full_hash) as u64;

    // an all-zero buffer for the game constructor
    #[allow(unsafe_code)]
    pub fn zeroed() -> Self {
        // SAFETY: every field is plain data for which all-zero bytes are valid (an empty small string)
        unsafe { std::mem::zeroed() }
    }
}

const _: () = assert!(offset_of!(ResourceLocationLayout, path) == 0x08);
const _: () = assert!(offset_of!(ResourceLocationLayout, _headroom) == 0x38);
