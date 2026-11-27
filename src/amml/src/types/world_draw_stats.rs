// what one world draw cost: the mesh builds and the draws in microseconds, the meshes built, the quads converted for them and the meshes drawn
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WorldDrawStats {
    pub build_us: f32,
    pub draw_us: f32,
    pub meshes_built: u32,
    pub quads_converted: u32,
    pub meshes_drawn: u32,
}
