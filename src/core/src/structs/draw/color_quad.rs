// corners are local to the planner origin, in wall-plane winding order
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorQuad {
    pub corners: [[f32; 3]; 4],
    pub rgba: [u8; 4],
}
