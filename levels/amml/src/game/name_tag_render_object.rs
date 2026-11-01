use amml_game::EntryId;

use crate::game::GameMirror;
use crate::types::LevelsEntry;

/// Layout mirror of the game's `NameTagRenderObject` (26.51 header: the text 0x20, two material pointers, two
/// colours, the position, the scale, a variant, an optional rotation and the line height). The mod reads no member:
/// the game builds each object through `_extractRenderTextObject` and the mod moves them bit for bit, so the members
/// are one span
#[repr(C, align(8))]
pub struct NameTagRenderObject {
    _members: [u8; 0x90], // 0x00 mNameTag .. mLineHeight
}

impl GameMirror for NameTagRenderObject {
    const SIZE_ENTRY: EntryId = LevelsEntry::NAME_TAG_SIZE;
}
