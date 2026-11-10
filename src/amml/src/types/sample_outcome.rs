// what sampling one column gave
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleOutcome {
    // the surfaces were written to the output; the count can be 0
    Sampled(usize),
    // the chunk is not loaded or its bounds are unreadable: nothing is submitted for the column
    Skipped,
}
