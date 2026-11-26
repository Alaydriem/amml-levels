#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InfoConnection {
    Local,
    Realm,
    Server,
    #[default]
    Unknown,
}
