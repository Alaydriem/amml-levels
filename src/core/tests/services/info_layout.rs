use levels_core::services::InfoLayout;
use levels_core::structs::info::InfoCorner;

#[test]
fn top_left_layout() {
    let p = InfoLayout::place(400.0, 300.0, 100.0, 3, InfoCorner::TopLeft, 0.0);
    assert_eq!((p.background.x, p.background.y, p.background.w, p.background.h), (2.0, 2.0, 106.0, 36.0));
    assert_eq!(p.text_x, 5.0);
    assert_eq!(p.first_line_y, 5.0);
    assert_eq!(p.visible_lines, 3);
    assert!(!p.right_aligned);
}

#[test]
fn top_right_layout() {
    let p = InfoLayout::place(400.0, 300.0, 100.0, 3, InfoCorner::TopRight, 0.0);
    assert_eq!(p.background.x, 292.0);
    assert!(p.right_aligned);
    assert_eq!(p.text_x, 395.0);
}

#[test]
fn panel_taller_than_screen_is_clamped() {
    let p = InfoLayout::place(400.0, 100.0, 100.0, 24, InfoCorner::TopLeft, 0.0);
    assert_eq!(p.visible_lines, 9);
    assert_eq!(p.background.h, 96.0);
}

#[test]
fn top_offset_moves_the_panel_down() {
    let p = InfoLayout::place(400.0, 300.0, 100.0, 3, InfoCorner::TopLeft, 24.0);
    assert_eq!(p.background.y, 26.0);
    assert_eq!(p.first_line_y, 29.0);
    assert_eq!(p.background.x, 2.0);
}

#[test]
fn top_offset_reduces_the_lines_that_fit() {
    let p = InfoLayout::place(400.0, 100.0, 100.0, 24, InfoCorner::TopLeft, 24.0);
    assert_eq!(p.visible_lines, 6);
    assert!(p.background.y + p.background.h <= 100.0);
}

#[test]
fn top_offset_past_the_screen_shows_no_lines() {
    let p = InfoLayout::place(400.0, 100.0, 100.0, 24, InfoCorner::TopLeft, 200.0);
    assert_eq!(p.visible_lines, 0);
}
