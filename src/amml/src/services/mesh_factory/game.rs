use amml_game::GameHeap;
use levels_core::structs::draw::ColorQuad;

use crate::game::{ScreenContext, Tessellator, WhiteTexture};
use crate::services::{GameStrings, QuadWriter};
use crate::types::{BuildStall, GameString, MeshAttempt, MeshBuild, WorldFrame, WorldMesh, WorldSlot};

/// Builds and draws the world meshes through the game: quads into the screen's tessellator, the mesh drawn with the
/// level renderer's wireframe material and the mod's white texture. The texture is a render-device object, loaded on the first frame
/// and released with the meshes on the render thread. The texture path and the mesh debug names reach the game as
/// game-heap copies, never as pointers into the mod's image
pub struct GameMeshFactory {
    writer: QuadWriter,
    strings: GameStrings,
    texture: Option<WhiteTexture>,
    // the load failed: no new try until the next release
    texture_failed: bool,
}

impl GameMeshFactory {
    // `allocate` is the game heap's allocator, for the texts the game objects keep
    pub fn new(allocate: unsafe extern "C" fn(usize) -> *mut u8) -> Self {
        Self { writer: QuadWriter::new(), strings: GameStrings::new(allocate), texture: None, texture_failed: false }
    }

    /// Loads the white texture when it is not loaded; true when it is ready
    ///
    /// # Safety
    /// `frame` must hold the live screen and client of the running render call. Render thread only
    #[allow(unsafe_code)]
    pub unsafe fn prepare(&mut self, frame: &WorldFrame<'_>) -> bool {
        if self.texture.is_some() {
            return true;
        }
        if self.texture_failed || frame.client.is_null() {
            return false;
        }
        let path = self.strings.get(GameString::WhitePath);
        // SAFETY: the caller guarantees a live client and the render thread
        self.texture = path.and_then(|path| unsafe { WhiteTexture::load(frame.client, frame.calls, path) });
        if self.texture.is_none() {
            self.texture_failed = true;
            curia::error!("the white texture could not be made; chunk borders and spawn spheres are not drawn", {
                path: GameString::WhitePath.text()
            });
        }
        self.texture.is_some()
    }

    /// Builds one mesh from `quads` on the screen's tessellator. An empty mesh from `end` (the tessellator drops
    /// what it holds while its discard flag is set) is a failed build, which the slot tries again
    ///
    /// # Safety
    /// As for `prepare`
    #[allow(unsafe_code)]
    pub unsafe fn build(&mut self, frame: &WorldFrame<'_>, quads: &[ColorQuad], slot: WorldSlot) -> MeshAttempt {
        // SAFETY: the caller guarantees the live ScreenContext of the call
        let Some(screen) = (unsafe { ScreenContext::from_raw(frame.screen, frame.calls) }) else {
            return MeshAttempt::Busy(BuildStall::NoScreen);
        };
        let Some(pointer) = screen.mesh_tessellator() else { return MeshAttempt::Busy(BuildStall::NoTessellator) };
        // SAFETY: the tessellator the ScreenContext holds, live for the call
        let Some(mut tessellator) = (unsafe { Tessellator::from_raw(pointer, frame.calls) }) else {
            return MeshAttempt::Busy(BuildStall::NoTessellator);
        };
        let Some(heap) = GameHeap::msvc() else { return MeshAttempt::Busy(BuildStall::NoHeap) };
        let Some(name) = self.strings.get(GameString::mesh(slot)) else { return MeshAttempt::Busy(BuildStall::NoHeap) };
        let reserve = i32::try_from(quads.len() * self.writer.vertices_per_quad()).unwrap_or(i32::MAX);
        let writer = &mut self.writer;
        // SAFETY: the heap is the game's, the one the tessellator's vectors came from
        let built = tessellator.build_quads(reserve, name, |open| unsafe { writer.write(open, &heap, quads) });
        match built {
            Err(stall) => MeshAttempt::Busy(stall),
            Ok(MeshBuild { filled: false, vertices, .. }) => MeshAttempt::Failed { vertices },
            Ok(MeshBuild { mesh: None, .. }) => MeshAttempt::Busy(BuildStall::NoMesh),
            Ok(MeshBuild { mesh: Some(mesh), vertices, .. }) if mesh.is_valid() => {
                MeshAttempt::Built(WorldMesh::Game { mesh, vertices })
            }
            Ok(MeshBuild { vertices, .. }) => MeshAttempt::Failed { vertices },
        }
    }

    /// Draws `mesh` at `offset` from the camera, turned `yaw_degrees` about the vertical axis
    ///
    /// # Safety
    /// As for `prepare`; `frame.material` must be null or a live `mce::MaterialPtr`
    #[allow(unsafe_code)]
    pub unsafe fn draw(&self, frame: &WorldFrame<'_>, mesh: &WorldMesh, offset: [f32; 3], yaw_degrees: f32) {
        let WorldMesh::Game { mesh, .. } = mesh else { return };
        let Some(texture) = self.texture.as_ref() else {
            return;
        };
        // SAFETY: the caller guarantees the live ScreenContext of the call
        let Some(screen) = (unsafe { ScreenContext::from_raw(frame.screen, frame.calls) }) else {
            return;
        };
        let Some(_pushed) = screen.push_world_yaw(offset, yaw_degrees) else {
            return;
        };
        screen.set_world_constants();
        // SAFETY: the material is live per the caller, and the variant borrows the uses the texture holds
        screen.with_white_shader_color(|| unsafe { mesh.render(&screen, frame.material, &texture.variant()) });
    }

    /// sets the sides of the quads the next builds write; true when that changes them
    pub fn set_two_sided(&mut self, two_sided: bool) -> bool {
        self.writer.set_two_sided(two_sided)
    }

    // drops the texture; the meshes drawn with it must be gone first
    pub fn release(&mut self) {
        self.texture = None;
        self.texture_failed = false;
    }
}
