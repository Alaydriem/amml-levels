use amml_msvc::{MsvcHeap, MsvcString, MsvcVec};

use crate::types::WidthSource;

// the sixteen texts "0" to "15" the labels show, with the width of each in the game font. A text is an MSVC string
// and the widths an MSVC vector of one int, as `_extractRenderTextObject` takes them by reference. They are made once
// and kept
pub struct LabelText {
    texts: Vec<MsvcString>,
    widths: Vec<MsvcVec<i32>>,
    source: Option<WidthSource>,
}

impl LabelText {
    pub const VALUES: usize = 16;

    // the width of one digit in the fixed table; the labels have no background, so the width only places the text
    const FIXED_DIGIT_WIDTH: i32 = 6;

    pub const fn new() -> Self {
        Self { texts: Vec::new(), widths: Vec::new(), source: None }
    }

    pub fn is_ready(&self) -> bool {
        self.source.is_some()
    }

    // where the widths came from; `None` before `prepare`
    pub fn source(&self) -> Option<WidthSource> {
        self.source
    }

    // the digits of value `i` as the text the label shows
    pub fn label(value: usize) -> String {
        value.min(Self::VALUES - 1).to_string()
    }

    /// makes the texts and widths once. `measure` gets the sixteen texts and gives their widths in the game font; it
    /// gives `None`, or a wrong count, when the font cannot be read, and the fixed table is used then. `None` when
    /// the game heap cannot make the strings
    ///
    /// # Safety
    /// The heap's functions must be the game's `operator new` and `operator delete`
    #[allow(unsafe_code)]
    pub unsafe fn prepare(
        &mut self,
        heap: &MsvcHeap,
        measure: impl FnOnce(&[&str]) -> Option<Vec<i32>>,
    ) -> Option<WidthSource> {
        if self.source.is_some() {
            return self.source;
        }
        let labels: Vec<String> = (0..Self::VALUES).map(Self::label).collect();
        let views: Vec<&str> = labels.iter().map(String::as_str).collect();
        let measured = measure(&views).filter(|widths| widths.len() == Self::VALUES);
        let source = if measured.is_some() { WidthSource::Font } else { WidthSource::FixedTable };
        let widths = measured.unwrap_or_else(|| labels.iter().map(|label| Self::fixed_width(label)).collect());
        let mut texts = Vec::with_capacity(Self::VALUES);
        let mut vectors = Vec::with_capacity(Self::VALUES);
        for (label, width) in labels.iter().zip(widths) {
            // SAFETY: the caller guarantees the heap
            texts.push(unsafe { MsvcString::new_in(heap, label) }.ok()?);
            // SAFETY: as above
            vectors.push(unsafe { MsvcVec::from_slice_in(heap, &[width]) }.ok()?);
        }
        self.texts = texts;
        self.widths = vectors;
        self.source = Some(source);
        self.source
    }

    // true when every one of `values` has a text and a width vector. Before `prepare` none has
    pub fn covers(&self, values: impl IntoIterator<Item = u8>) -> bool {
        values.into_iter().all(|value| self.text(value).is_some() && self.widths(value).is_some())
    }

    pub fn text(&self, value: u8) -> Option<&MsvcString> {
        self.texts.get(usize::from(value).min(Self::VALUES - 1))
    }

    pub fn widths(&self, value: u8) -> Option<&MsvcVec<i32>> {
        self.widths.get(usize::from(value).min(Self::VALUES - 1))
    }

    // the width of `label` from the fixed table
    pub fn fixed_width(label: &str) -> i32 {
        i32::try_from(label.len()).unwrap_or(0) * Self::FIXED_DIGIT_WIDTH
    }
}

impl Default for LabelText {
    fn default() -> Self {
        Self::new()
    }
}
