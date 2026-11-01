// what one name-tag extraction did to the game's list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawOutcome {
    // labels were appended
    Drew(usize),
    // the overlay had no labels this frame: the game's list is untouched
    NoLabels,
    // the list was left as the game made it: the layout is not the expected one, the mod is off, or a game object the
    // draw needs is missing
    Skipped,
}
