use levels_core::structs::info::InfoLines;

#[test]
fn lines_stop_at_max() {
    let mut lines = InfoLines::new();
    let given = (0..30).filter(|_| lines.next_line().is_some()).count();
    assert_eq!(given, 24);
    assert_eq!(lines.len(), 24);
}
