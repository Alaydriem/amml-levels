use serde::Deserialize;

// how the info key shows the panel: press to show and press to hide, or shown while the key is down
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfoMode {
    #[default]
    Toggle,
    Hold,
}
