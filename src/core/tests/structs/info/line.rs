use core::fmt::Write;
use levels_core::structs::info::InfoLine;

#[test]
fn long_text_is_cut_at_char_boundary() {
    let mut line = InfoLine::new();
    line.write_str(&"a".repeat(95)).unwrap();
    line.write_str("é").unwrap();
    assert_eq!(line.as_str().len(), 95);
    line.write_str("b").unwrap();
    assert_eq!(line.as_str().len(), 95);
}
