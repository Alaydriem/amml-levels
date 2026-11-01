// what one world draw cost: the mesh builds and the draws in microseconds, and the meshes built
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WorldDrawStats {
    pub build_us: f32,
    pub draw_us: f32,
    pub meshes_built: u32,
}
