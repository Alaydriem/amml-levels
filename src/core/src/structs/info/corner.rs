use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfoCorner {
    #[default]
    TopLeft,
    TopRight,
}
