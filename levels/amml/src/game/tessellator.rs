use std::ffi::c_void;
use std::ptr::NonNull;

use crate::game::{MeshDataLayout, OpenBuild, OwnedMesh, TessellatorLayout};
use crate::types::{BuildStall, MeshBuild, MeshCalls, PrimitiveMode};

// void Tessellator::begin(DebugContextCallback, mce::PrimitiveMode, int reserved, bool): this, the empty callback
// struct in one byte, the mode, the vertex count to reserve, then a flag passed as false
type BeginFn = unsafe extern "C" fn(*mut c_void, u8, u8, i32, u8);
// mce::Mesh Tessellator::end(UploadMode, std::string_view, SupplementaryFieldAutoGenerationMode): this, the hidden
// Mesh buffer, the upload mode, a pointer to the debug name view, then the 2-byte mode; the buffer comes back in RAX
type EndFn = unsafe extern "C" fn(*mut c_void, *mut c_void, i32, *const [usize; 2], u16) -> *mut c_void;
// void Tessellator::clear()
type ClearFn = unsafe extern "C" fn(*mut c_void);

/// The game's `Tessellator`, the one a `ScreenContext` holds. Render thread only
pub struct Tessellator {
    ptr: NonNull<TessellatorLayout>,
    calls: MeshCalls,
}

impl Tessellator {
    // `Tessellator::UploadMode::Buffered`: the mesh is uploaded on its first draw
    const UPLOAD_BUFFERED: i32 = 0;
    // `Tessellator::UploadMode::Never`: the mesh is never uploaded, for a mesh that is dropped at once
    const UPLOAD_NEVER: i32 = 2;
    // `SupplementaryFieldAutoGenerationMode::None`
    const NO_SUPPLEMENTARY_FIELDS: u16 = 0;

    /// Wraps `ptr`; `None` for a null pointer
    ///
    /// # Safety
    /// `ptr` must be null or a live `Tessellator` of the build `calls` was resolved for, laid out as
    /// `TessellatorLayout`, and must stay live while the wrapper is used
    #[allow(unsafe_code)]
    pub unsafe fn from_raw(ptr: *mut c_void, calls: &MeshCalls) -> Option<Self> {
        NonNull::new(ptr.cast::<TessellatorLayout>()).map(|ptr| Self { ptr, calls: *calls })
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr().cast()
    }

    // whether a begin is open: `mTessellating`
    #[allow(unsafe_code)]
    pub fn is_tessellating(&self) -> bool {
        // SAFETY: `from_raw` guarantees a live Tessellator laid out as the mirror
        unsafe { (*self.ptr.as_ptr()).tessellating }
    }

    // begins a quad list; the game does nothing while a begin is open
    #[allow(unsafe_code)]
    pub fn begin_quads(&mut self, reserve: i32) {
        // SAFETY: the entry is the address of `Tessellator::begin` with the signature of `BeginFn`
        let begin: BeginFn = unsafe { std::mem::transmute(self.calls.tessellator_begin) };
        // SAFETY: `from_raw` guarantees a live Tessellator
        unsafe { begin(self.as_ptr(), 0, PrimitiveMode::QuadList as u8, reserve, 0) };
    }

    // ends the open begin into a mesh that is uploaded on its first draw; `None`, with no call, when no begin is open
    pub fn end(&mut self, debug_name: &str) -> Option<OwnedMesh> {
        self.end_with(Self::UPLOAD_BUFFERED, debug_name)
    }

    // ends the open begin into a mesh that is never uploaded, for one the caller drops at once; `None`, with no call,
    // when no begin is open
    pub fn end_unuploaded(&mut self, debug_name: &str) -> Option<OwnedMesh> {
        self.end_with(Self::UPLOAD_NEVER, debug_name)
    }

    #[allow(unsafe_code)]
    fn end_with(&mut self, upload: i32, debug_name: &str) -> Option<OwnedMesh> {
        if !self.is_tessellating() {
            return None;
        }
        // SAFETY: the entry is the address of `Tessellator::end` with the signature of `EndFn`
        let end: EndFn = unsafe { std::mem::transmute(self.calls.tessellator_end) };
        let name = [debug_name.as_ptr() as usize, debug_name.len()];
        let mut mesh = OwnedMesh::uninit(&self.calls);
        // SAFETY: a live Tessellator with an open begin; the buffer holds the larger mesh mirror, 8-aligned, and
        // `end` constructs the mesh in it; the name view lives for the call
        unsafe { end(self.as_ptr(), mesh.as_mut_ptr().cast(), upload, &name, Self::NO_SUPPLEMENTARY_FIELDS) };
        // SAFETY: `end` constructed a Mesh in the buffer
        Some(unsafe { mesh.assume_init() })
    }

    // drops what the open begin collected; the game also closes the begin
    #[allow(unsafe_code)]
    pub fn clear(&mut self) {
        // SAFETY: the entry is the address of `Tessellator::clear` with the signature of `ClearFn`
        let clear: ClearFn = unsafe { std::mem::transmute(self.calls.tessellator_clear) };
        // SAFETY: `from_raw` guarantees a live Tessellator
        unsafe { clear(self.as_ptr()) };
    }

    // the mesh data a build fills: `mMeshData`
    #[allow(unsafe_code)]
    pub fn mesh_data(&mut self) -> &mut MeshDataLayout {
        // SAFETY: `from_raw` guarantees a live Tessellator laid out as the mirror; the borrow ends with the wrapper's
        unsafe { &mut (*self.ptr.as_ptr()).mesh_data }
    }

    // `mCount`, the vertex count `end` uploads
    #[allow(unsafe_code)]
    pub fn count(&self) -> u32 {
        // SAFETY: `from_raw` guarantees a live Tessellator laid out as the mirror
        unsafe { (*self.ptr.as_ptr()).count }
    }

    // sets `mCount`, for vertices written into the mesh data by hand
    #[allow(unsafe_code)]
    pub fn set_count(&mut self, count: u32) {
        // SAFETY: `from_raw` guarantees a live Tessellator laid out as the mirror
        unsafe { (*self.ptr.as_ptr()).count = count };
    }

    // the vertices the tessellator holds: the length of `mMeshData.mPositions`
    #[allow(unsafe_code)]
    pub fn vertex_count(&self) -> usize {
        // SAFETY: `from_raw` guarantees a live Tessellator laid out as the mirror
        unsafe { (*self.ptr.as_ptr()).mesh_data.positions.len() }
    }

    /// Begins a quad list, runs `fill` and always ends it, also when `fill` panics. A fill that returns false is
    /// cleared from the tessellator first, then the begin ends into a mesh that is never uploaded and is dropped. An
    /// error, with nothing written, when a begin is already open or `begin` did not open one
    pub fn build_quads(
        &mut self,
        reserve: i32,
        debug_name: &str,
        fill: impl FnOnce(&mut Tessellator) -> bool,
    ) -> Result<MeshBuild, BuildStall> {
        if self.is_tessellating() {
            return Err(BuildStall::TessellatorOpen);
        }
        self.begin_quads(reserve);
        if !self.is_tessellating() {
            return Err(BuildStall::BeginRefused);
        }
        let mut open = OpenBuild::new(self, debug_name);
        let filled = fill(open.tessellator());
        let vertices = open.tessellator().vertex_count();
        let mesh = if filled {
            open.finish()
        } else {
            open.tessellator().clear();
            open.discard();
            None
        };
        Ok(MeshBuild { filled, vertices, mesh })
    }
}
