use super::PanelRect;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InfoPlacement {
    pub background: PanelRect,
    pub text_x: f32,
    pub first_line_y: f32,
    pub line_height: f32,
    pub right_aligned: bool,
    pub visible_lines: usize,
}
