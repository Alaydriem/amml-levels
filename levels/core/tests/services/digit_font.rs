use levels_core::services::DigitFont;

#[test]
fn digit_runs_match_the_glyph_table() {
    assert_eq!(DigitFont::runs(0).len(), 8);
    assert_eq!(DigitFont::runs(8).len(), 7);
    assert_eq!(DigitFont::runs(1), &[(0, 1, 1), (1, 0, 2), (2, 1, 1), (3, 1, 1), (4, 0, 3)]);
    for digit in 0..=9 {
        assert!(DigitFont::runs(digit).len() <= DigitFont::MAX_RUNS_PER_DIGIT);
    }
}
