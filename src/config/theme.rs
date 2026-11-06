use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Theme {
    pub name: String,
    pub header: String,
    pub border: String,
    pub accent: String,
}

impl Theme {
    pub fn default_theme() -> Self {
        Self {
            name: "default".to_string(),
            header: "white".to_string(),
            border: "cyan".to_string(),
            accent: "green".to_string(),
        }
    }
}
