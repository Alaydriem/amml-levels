// one number to draw: the top-face centre in world space, the light value and its RGBA color
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Label {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub value: u8,
    pub color: [f32; 4],
}
