use amml_api::amml_abi::ScreenRender;
use amml_api::{Color, Rect, Screen, ScreenRenderExt, TextAlign};
use levels_core::services::InfoPanel;

// draws the info panel core places on the first HUD view of each frame, on the game thread
pub struct InfoDrawer {
    // the frame the panel was last drawn in
    last_frame: Option<u64>,
}

impl InfoDrawer {
    pub fn new() -> Self {
        Self { last_frame: None }
    }

    pub fn draw(
        &mut self,
        event: &ScreenRender,
        top_screen: Option<Screen>,
        game_thread: bool,
        panel: &InfoPanel,
        frame: u64,
    ) {
        if !panel.is_shown() || top_screen != Some(Screen::Hud) || !game_thread || self.last_frame == Some(frame) {
            return;
        }
        let (Some(view), Some(draw)) = (event.view(), event.draw()) else { return };
        let Some(size) = view.size() else { return };
        self.last_frame = Some(frame);
        let lines = panel.lines();
        let visible = panel.placement(size.width as f32, size.height as f32, 0.0).visible_lines;
        if visible == 0 {
            return;
        }
        let mut widest = 0.0f32;
        for line in lines.iter().take(visible) {
            let Some(width) = draw.text_width(line.as_str()) else { return };
            widest = widest.max(width);
        }
        let placement = panel.placement(size.width as f32, size.height as f32, widest);
        let background = placement.background;
        let rect = Rect { x: background.x, y: background.y, width: background.w, height: background.h };
        let [red, green, blue] = panel.background_color();
        let _ = draw.fill_rect(rect, Color { r: red, g: green, b: blue }, panel.background_alpha());
        let (x, align) = if placement.right_aligned {
            (placement.text_x - widest, TextAlign::Right)
        } else {
            (placement.text_x, TextAlign::Left)
        };
        let mut y = placement.first_line_y;
        for line in lines.iter().take(placement.visible_lines) {
            let [r, g, b] = line.color();
            let rect = Rect { x, y, width: widest, height: placement.line_height };
            let _ = draw.text(line.as_str(), rect, Color { r, g, b }, panel.text_alpha(), align);
            y += placement.line_height;
        }
    }
}

impl Default for InfoDrawer {
    fn default() -> Self {
        Self::new()
    }
}
