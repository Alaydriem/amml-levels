use crate::structs::info::{InfoCorner, InfoPlacement, PanelRect};

pub struct InfoLayout;

impl InfoLayout {
    pub const MARGIN: f32 = 2.0;
    pub const PADDING: f32 = 3.0;
    pub const LINE_HEIGHT: f32 = 10.0;

    // `top` moves the panel down from the top edge, below the game's own coordinates text
    pub fn place(
        screen_w: f32,
        screen_h: f32,
        widest: f32,
        lines: usize,
        corner: InfoCorner,
        top: f32,
    ) -> InfoPlacement {
        let top_y = top + Self::MARGIN;
        let usable = (screen_h - top - 2.0 * Self::MARGIN - 2.0 * Self::PADDING) / Self::LINE_HEIGHT;
        let fit = if usable > 0.0 { usable.floor() as usize } else { 0 };
        let visible_lines = lines.min(fit);
        let w = widest + 2.0 * Self::PADDING;
        let h = visible_lines as f32 * Self::LINE_HEIGHT + 2.0 * Self::PADDING;
        let right_aligned = corner == InfoCorner::TopRight;
        let x = if right_aligned { screen_w - Self::MARGIN - w } else { Self::MARGIN };
        let text_x = if right_aligned { screen_w - Self::MARGIN - Self::PADDING } else { Self::MARGIN + Self::PADDING };
        InfoPlacement {
            background: PanelRect { x, y: top_y, w, h },
            text_x,
            first_line_y: top_y + Self::PADDING,
            line_height: Self::LINE_HEIGHT,
            right_aligned,
            visible_lines,
        }
    }
}
