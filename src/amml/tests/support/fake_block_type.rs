use std::mem::MaybeUninit;

use amml_game::BlockTypePointerLayout;
use amml_msvc::MsvcString;

use super::{FakeHeap, FakeVtable};

type ShapeFn =
    unsafe extern "C" fn(*mut u8, *mut [f32; 6], *const u8, *const u8, *const [i32; 3], *const u8) -> *mut [f32; 6];

// a fake `BlockType` laid out as the 26.51 mirror (the vtable at 0, the full name at E0h), then the collision box its
// virtual returns
#[repr(C)]
pub struct FakeBlockType {
    pub layout: BlockTypePointerLayout,
    pub shape: [f32; 6],
}

impl FakeBlockType {
    // the slot of `getCollisionShape` in the fake vtable
    pub const SHAPE_SLOT: u64 = 1;

    // `name` is at most 15 bytes, so it needs no heap block
    pub fn new(name: &str, shape: [f32; 6]) -> Box<Self> {
        let function: ShapeFn = Self::get_collision_shape;
        let vtable = FakeVtable::leak(&[function as usize]);
        // SAFETY: the mirror holds raw pointers, integers and MSVC strings, for which all zero bytes are valid (an
        // empty string that owns no block)
        let mut layout: BlockTypePointerLayout = unsafe { MaybeUninit::zeroed().assume_init() };
        // SAFETY: the heap functions are valid allocate and free functions, and a short name uses no block
        layout.full_name.text = unsafe { MsvcString::new_in(&FakeHeap::msvc(), name) }.unwrap();
        let mut fake = Box::new(Self { layout, shape });
        // SAFETY: the vtable pointer is the first qword of the mirror, a private member
        unsafe { (&raw mut fake.layout).cast::<*const usize>().write(vtable) };
        fake
    }

    unsafe extern "C" fn get_collision_shape(
        this: *mut u8,
        out: *mut [f32; 6],
        _block: *const u8,
        _region: *const u8,
        _pos: *const [i32; 3],
        _entity: *const u8,
    ) -> *mut [f32; 6] {
        // SAFETY: the fake is only reached through a FakeBlockType, and `out` is the caller buffer
        unsafe { *out = (*this.cast::<FakeBlockType>()).shape };
        out
    }
}
