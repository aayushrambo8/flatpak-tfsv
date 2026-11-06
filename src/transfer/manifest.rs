use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransferManifest {
    pub source: String,
    pub destination: String,
    pub mode: String,
}

impl TransferManifest {
    pub fn new(source: impl Into<String>, destination: impl Into<String>, mode: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            destination: destination.into(),
            mode: mode.into(),
        }
    }
}
