use serde::Deserialize;

use crate::error::QqError;

#[derive(Debug, Clone, Deserialize)]
pub struct QQConfig {
    pub app_id: String,
    pub client_secret: String,
}

impl QQConfig {
    pub fn from_toml_value(value: &toml::Value) -> Result<Self, QqError> {
        let json = serde_json::to_value(value)?;
        Ok(serde_json::from_value(json)?)
    }
}
