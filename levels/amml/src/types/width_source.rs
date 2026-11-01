// where the label widths came from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthSource {
    // measured in the game font
    Font,
    // the fixed table, because the font reads are off
    FixedTable,
}
