use std::ffi::c_void;
use std::mem::MaybeUninit;

use amml_game::BlockLayout;

use super::FakeBlockType;

// a fake `Block` laid out as the mirror (the block type pointer at 68h), then whether the top face gives full support
#[repr(C)]
pub struct FakeBlock {
    pub layout: BlockLayout,
    pub full_support: u8,
}

impl FakeBlock {
    pub fn new(block_type: *mut FakeBlockType, full_support: bool) -> Box<Self> {
        // SAFETY: the mirror holds raw pointers and integers, for which all zero bytes are valid
        let mut layout: BlockLayout = unsafe { MaybeUninit::zeroed().assume_init() };
        layout.block_type = block_type.cast();
        Box::new(Self { layout, full_support: u8::from(full_support) })
    }

    pub fn ptr(&mut self) -> *mut c_void {
        (&raw mut *self).cast()
    }

    // `bool Block::canProvideFullSupport(uchar) const`
    pub unsafe extern "C" fn can_provide_full_support(this: *mut c_void, _face: u8) -> u8 {
        // SAFETY: the fake is only reached through a FakeBlock
        unsafe { (*this.cast::<FakeBlock>()).full_support }
    }
}
