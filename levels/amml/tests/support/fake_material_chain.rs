use std::ffi::c_void;

use alaydriem_levels::types::LevelsEntry;
use amml_game::{EntryId, GameEntries};

use crate::support::FakeResolver;

// a LevelRenderer -> LevelRendererPlayer -> MaterialPtr -> RenderMaterialInfo -> RenderMaterial chain in plain memory
pub struct FakeMaterialChain {
    level_renderer: Vec<u64>,
    player: Vec<u64>,
    info: Vec<u64>,
    material: Vec<u64>,
    // materials the info no longer points at; kept allocated so a patch may still restore them
    retired: Vec<Vec<u64>>,
}

impl FakeMaterialChain {
    pub const PLAYER_AT: usize = 0x480;
    pub const MATERIAL_AT: usize = 0x2e8;
    pub const INFO_MATERIAL_AT: usize = 0x40;
    pub const BLEND_AT: usize = 0x185;
    pub const SOURCE_AT: usize = 0x180;
    pub const DESTINATION_AT: usize = 0x181;
    pub const DEPTH_AT: usize = 0x193;
    const MATERIAL_WORDS: usize = 0x60;

    // a full chain whose blend byte is 0, blend factors (One 3, Zero 2) and depth-write byte 1
    pub fn new() -> Self {
        let mut chain = Self {
            level_renderer: vec![0; 0x100],
            player: vec![0; 0x100],
            info: vec![0; 0x20],
            material: Self::fresh_material(),
            retired: Vec::new(),
        };
        let player = chain.player.as_ptr() as u64;
        let info = chain.info.as_ptr() as u64;
        let material = chain.material.as_ptr() as u64;
        chain.level_renderer[Self::PLAYER_AT / 8] = player;
        chain.player[Self::MATERIAL_AT / 8] = info;
        chain.info[Self::INFO_MATERIAL_AT / 8] = material;
        chain
    }

    pub fn values() -> Vec<(EntryId, u64)> {
        vec![
            (LevelsEntry::LEVEL_RENDERER_PLAYER, Self::PLAYER_AT as u64),
            (LevelsEntry::WIREFRAME_MATERIAL, Self::MATERIAL_AT as u64),
            (LevelsEntry::MATERIAL_INFO_MATERIAL, Self::INFO_MATERIAL_AT as u64),
            (LevelsEntry::MATERIAL_ENABLE_BLEND, Self::BLEND_AT as u64),
            (LevelsEntry::MATERIAL_DEPTH_WRITE, Self::DEPTH_AT as u64),
            (LevelsEntry::MATERIAL_BLEND_SOURCE, Self::SOURCE_AT as u64),
            (LevelsEntry::MATERIAL_BLEND_DESTINATION, Self::DESTINATION_AT as u64),
        ]
    }

    pub fn entries() -> GameEntries {
        FakeResolver::entries(&Self::values())
    }

    pub fn level_renderer(&self) -> *const c_void {
        self.level_renderer.as_ptr().cast()
    }

    // the address of the LevelRendererPlayer block
    pub fn player_address(&self) -> usize {
        self.player.as_ptr() as usize
    }

    // the address of the current RenderMaterial block
    pub fn material_address(&self) -> usize {
        self.material.as_ptr() as usize
    }

    pub fn null_player(&mut self) {
        self.level_renderer[Self::PLAYER_AT / 8] = 0;
    }

    pub fn null_info(&mut self) {
        self.player[Self::MATERIAL_AT / 8] = 0;
    }

    pub fn null_material(&mut self) {
        self.info[Self::INFO_MATERIAL_AT / 8] = 0;
    }

    // points the info at a new RenderMaterial with the starting bytes; the old one stays allocated
    pub fn swap_material(&mut self) {
        let next = Self::fresh_material();
        self.info[Self::INFO_MATERIAL_AT / 8] = next.as_ptr() as u64;
        let old = std::mem::replace(&mut self.material, next);
        self.retired.push(old);
    }

    // a byte of the current RenderMaterial
    pub fn byte(&self, offset: usize) -> u8 {
        Self::byte_of(&self.material, offset)
    }

    // a byte of the RenderMaterial the info pointed at before the last swap
    pub fn retired_byte(&self, offset: usize) -> u8 {
        Self::byte_of(&self.retired[0], offset)
    }

    pub fn set_byte(&mut self, offset: usize, value: u8) {
        // SAFETY: the offset is inside the block
        unsafe { self.material.as_mut_ptr().cast::<u8>().add(offset).write(value) }
    }

    fn byte_of(block: &[u64], offset: usize) -> u8 {
        // SAFETY: the offset is inside the block
        unsafe { block.as_ptr().cast::<u8>().add(offset).read() }
    }

    fn fresh_material() -> Vec<u64> {
        let mut block = vec![0u64; Self::MATERIAL_WORDS];
        // SAFETY: the offset is inside the block
        unsafe {
            let bytes = block.as_mut_ptr().cast::<u8>();
            bytes.add(Self::DEPTH_AT).write(1);
            bytes.add(Self::SOURCE_AT).write(3);
            bytes.add(Self::DESTINATION_AT).write(2);
        }
        block
    }
}
