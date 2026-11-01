// One marker drawn at a marker this frame. The center is in world coordinates, the spin in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkerInstance {
    pub center: [f32; 3],
    pub spin_degrees: f32,
}
