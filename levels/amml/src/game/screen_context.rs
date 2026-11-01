use std::ffi::c_void;
use std::mem::offset_of;
use std::ptr::NonNull;

use amml_game::game::MatrixStackLayout;
use amml_game::{CameraLayout, GameEntries, RawObject};
use amml_msvc::MemberCall;

use crate::game::{MatrixStackRefLayout, MeshContextLayout, WorldMatrixRef};
use crate::types::{LevelsEntry, MeshCalls};

// static void ActorShaderManager::setEntityConstants(ActorConstants&, RenderContext&, Color const& tileLight,
// Vec2 const& tileLightUV, glm::vec4 const& blockLight, Color const& overlay, Color const& change,
// Color const& change2, Color const& glint, Vec2 const& glintUVScale, Vec4 const& uvAnim, float uvOffset1,
// float uvOffset2, float uvRot1, float uvRot2)
type SetEntityConstantsFn = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *const [f32; 4],
    *const [f32; 2],
    *const [f32; 4],
    *const [f32; 4],
    *const [f32; 4],
    *const [f32; 4],
    *const [f32; 4],
    *const [f32; 2],
    *const [f32; 4],
    f32,
    f32,
    f32,
    f32,
);

/// The game's `ScreenContext` of one render call: its tessellator, its `MeshContext` base and, through that, the
/// camera's world matrix stack and the actor constants. Render thread, inside the frame it belongs to
pub struct ScreenContext {
    ptr: NonNull<c_void>,
    calls: MeshCalls,
}

impl ScreenContext {
    // where the world matrix stack sits in `mce::Camera`: after the view matrix stack, before the projection one
    const WORLD_STACK: usize = offset_of!(CameraLayout, projection_stack) - size_of::<MatrixStackLayout>();

    /// Wraps `ptr`; `None` for a null pointer
    ///
    /// # Safety
    /// `ptr` must be null or the live `ScreenContext` of the running render call, of the build `calls` was
    /// resolved for, and must stay live while the wrapper is used
    #[allow(unsafe_code)]
    pub unsafe fn from_raw(ptr: *mut c_void, calls: &MeshCalls) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self { ptr, calls: *calls })
    }

    /// the `Tessellator&` of the context, read as the pointer it is stored as. `None` when the entry is off, the
    /// context is null or the reference is null
    ///
    /// # Safety
    /// `context` must be null or a live ScreenContext of the game build the entries were resolved for
    #[allow(unsafe_code)]
    pub unsafe fn tessellator(context: *const c_void, entries: &GameEntries) -> Option<*mut c_void> {
        let offset = entries.get(LevelsEntry::SCREEN_CONTEXT_TESSELLATOR)?;
        if context.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees a live ScreenContext, and the entry is the offset of its `tessellator`
        // reference, which the game stores as a pointer
        unsafe { RawObject::pointer(context, offset) }
    }

    // the `Tessellator&` of the wrapped context; `None` when it is null
    #[allow(unsafe_code)]
    pub fn mesh_tessellator(&self) -> Option<*mut c_void> {
        // SAFETY: `from_raw` guarantees a live ScreenContext, and the entry is the offset of its `tessellator`
        // reference, stored as a pointer
        unsafe { RawObject::pointer(self.ptr.as_ptr(), self.calls.screen_tessellator) }
    }

    // the `mce::MeshContext&` base that `Mesh::renderMesh` takes, `ScreenContext.meshContext` into the context
    pub fn mesh_context(&self) -> *mut c_void {
        self.ptr.as_ptr().cast::<u8>().wrapping_add(self.calls.mesh_context as usize).cast()
    }

    #[allow(unsafe_code)]
    fn mesh_fields(&self) -> &MeshContextLayout {
        // SAFETY: `from_raw` guarantees a live ScreenContext, whose MeshContext base the mirror lays out
        unsafe { &*self.mesh_context().cast::<MeshContextLayout>() }
    }

    /// Pushes a copy of the world matrix times `translate(offset) * rotate_y(yaw_degrees)` on the camera's world
    /// matrix stack and marks the stack dirty: a mesh vertex `p` lands at `offset + R_y(yaw) * p`. The drop of the
    /// result pops it. `None` when the camera is null, or when the push gave no matrix (the push is popped again)
    #[allow(unsafe_code)]
    pub fn push_world_yaw(&self, offset: [f32; 3], yaw_degrees: f32) -> Option<WorldMatrixRef> {
        let camera = self.mesh_fields().camera;
        if camera.is_null() {
            return None;
        }
        let stack = camera.cast::<u8>().wrapping_add(Self::WORLD_STACK).cast::<c_void>();
        // SAFETY: the entry is the address of `MatrixStack::push(bool isSortOrigin)`, a member that returns its
        // 16-byte ref through the hidden pointer; `stack` is the live camera's world matrix stack
        let pushed: MatrixStackRefLayout =
            unsafe { MemberCall::call_sret1(self.calls.matrix_stack_push as *const c_void, stack, 1u8) };
        let reference = WorldMatrixRef::new(pushed, self.calls.matrix_stack_ref_dtor);
        let matrix = reference.matrix();
        if matrix.is_null() {
            return None;
        }
        // SAFETY: `push` gave the new top matrix of the stack, a glm::mat4 stored by columns
        Self::translate_yaw(unsafe { &mut *matrix }, offset, yaw_degrees);
        // SAFETY: the stack is live and laid out as the mirror; `_isDirty` tells the game the top changed
        unsafe { (*stack.cast::<MatrixStackLayout>()).is_dirty = true };
        Some(reference)
    }

    /// `columns` times `translate(offset) * rotate_y(yaw_degrees)`, for a glm::mat4 stored by columns
    pub fn translate_yaw(columns: &mut [[f32; 4]; 4], offset: [f32; 3], yaw_degrees: f32) {
        let moved: [f32; 4] = std::array::from_fn(|row| {
            columns[0][row] * offset[0] + columns[1][row] * offset[1] + columns[2][row] * offset[2]
        });
        for (value, add) in columns[3].iter_mut().zip(moved) {
            *value += add;
        }
        let (sin, cos) = yaw_degrees.to_radians().sin_cos();
        let x = columns[0];
        let z = columns[2];
        for row in 0..4 {
            columns[0][row] = x[row] * cos - z[row] * sin;
            columns[2][row] = x[row] * sin + z[row] * cos;
        }
    }

    /// Runs `draw` with the context's current shader colour set to opaque white, then puts the colour back; both
    /// writes mark it dirty. The name-tag material multiplies its output by this colour, and the game's name-tag pass
    /// leaves it at the colour of the last tag background, which is transparent for the light labels. A null colour
    /// pointer runs `draw` with the colour as it is
    #[allow(unsafe_code)]
    pub fn with_white_shader_color(&self, draw: impl FnOnce()) {
        let color = self.mesh_fields().current_shader_color;
        if color.is_null() {
            draw();
            return;
        }
        // SAFETY: the MeshContext's `ShaderColor&`, stored as a pointer, is the frame's live shared colour; this is
        // the render thread, which owns it during the call
        let saved = unsafe { (*color).color };
        // SAFETY: as above
        unsafe {
            (*color).color = [1.0; 4];
            (*color).dirty = true;
        }
        draw();
        // SAFETY: as above
        unsafe {
            (*color).color = saved;
            (*color).dirty = true;
        }
    }

    /// Writes the fixed actor constants of a world mesh draw: white tile light and block light, no overlay, no
    /// change colours, no glint, UV animation offset 0 and scale 1. Without them a draw takes the last actor's light
    #[allow(unsafe_code)]
    pub fn set_world_constants(&self) {
        let fields = self.mesh_fields();
        if fields.constant_buffers.is_null() || fields.render_context.is_null() {
            return;
        }
        let entity = fields.constant_buffers.cast::<u8>().wrapping_add(self.calls.entity_constant_buffer as usize);
        let white = [1.0f32; 4];
        let none = [0.0f32; 4];
        let one = [1.0f32; 2];
        let uv_anim = [0.0f32, 0.0, 1.0, 1.0];
        // SAFETY: the entry is the address of the static `ActorShaderManager::setEntityConstants`
        let set: SetEntityConstantsFn = unsafe { std::mem::transmute(self.calls.set_entity_constants) };
        // SAFETY: the frame's live entity constant buffer and render context, and values that live for the call
        unsafe {
            set(
                entity.cast(),
                fields.render_context,
                &white,
                &one,
                &white,
                &none,
                &none,
                &none,
                &none,
                &one,
                &uv_anim,
                0.0,
                0.0,
                0.0,
                0.0,
            )
        };
    }
}
