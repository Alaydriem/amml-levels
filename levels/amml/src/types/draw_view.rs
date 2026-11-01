use levels_core::structs::draw::Label;

// what to draw this frame: the labels, the rotation every label shares and the scale
#[derive(Debug, Clone, Copy)]
pub struct DrawView<'a> {
    pub labels: &'a [Label],
    // the quaternion x, y, z, w
    pub rotation: [f32; 4],
    pub scale: f32,
}
