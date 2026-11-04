const GLYPHS: [&[(u8, u8, u8)]; 10] = [
    &[(0, 0, 3), (1, 0, 1), (1, 2, 1), (2, 0, 1), (2, 2, 1), (3, 0, 1), (3, 2, 1), (4, 0, 3)],
    &[(0, 1, 1), (1, 0, 2), (2, 1, 1), (3, 1, 1), (4, 0, 3)],
    &[(0, 0, 3), (1, 2, 1), (2, 0, 3), (3, 0, 1), (4, 0, 3)],
    &[(0, 0, 3), (1, 2, 1), (2, 1, 2), (3, 2, 1), (4, 0, 3)],
    &[(0, 0, 1), (0, 2, 1), (1, 0, 1), (1, 2, 1), (2, 0, 3), (3, 2, 1), (4, 2, 1)],
    &[(0, 0, 3), (1, 0, 1), (2, 0, 3), (3, 2, 1), (4, 0, 3)],
    &[(0, 0, 3), (1, 0, 1), (2, 0, 3), (3, 0, 1), (3, 2, 1), (4, 0, 3)],
    &[(0, 0, 3), (1, 2, 1), (2, 1, 1), (3, 1, 1), (4, 1, 1)],
    &[(0, 0, 3), (1, 0, 1), (1, 2, 1), (2, 0, 3), (3, 0, 1), (3, 2, 1), (4, 0, 3)],
    &[(0, 0, 3), (1, 0, 1), (1, 2, 1), (2, 0, 3), (3, 2, 1), (4, 0, 3)],
];

// a 3 x 5 pixel digit font stored as horizontal runs
pub struct DigitFont;

impl DigitFont {
    pub const MAX_RUNS_PER_DIGIT: usize = 8;
    pub const WIDTH: u8 = 3;
    pub const HEIGHT: u8 = 5;

    // (row, first_column, length) runs of a digit, row 0 at the top; a value above 9 has no runs
    pub fn runs(digit: u8) -> &'static [(u8, u8, u8)] {
        GLYPHS.get(digit as usize).copied().unwrap_or(&[])
    }
}
