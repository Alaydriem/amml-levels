use amml_msvc::MsvcVec;

use crate::game::NameTagRenderObject;

/// Layout mirror of the game's `NameTagRenderObjectCollection` (26.51 header): one
/// `std::vector<NameTagRenderObject, LinearAllocator<..>>`, whose allocator (a pointer and a shared_ptr control
/// block) comes before the MSVC vector pointers
#[repr(C)]
pub struct NameTagRenderObjectCollection {
    _allocator: [u8; 0x10],                      // 0x00 mNameTags: LinearAllocator
    pub name_tags: MsvcVec<NameTagRenderObject>, // 0x10 mNameTags: first, last, end
}
