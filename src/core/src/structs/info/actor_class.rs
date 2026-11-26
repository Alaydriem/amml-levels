#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorClass {
    Player,
    Hostile,
    Passive,
    Other,
}
