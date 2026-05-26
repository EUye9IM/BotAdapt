use serde::Deserialize;

use crate::error::QqError;

#[derive(Debug, Clone, Deserialize)]
pub struct QQConfig {
    pub app_id: String,
    pub client_secret: String,
}

impl QQConfig {
    pub fn from_toml_table(value: &toml::Table) -> Result<Self, QqError> {
        Ok(value.clone().try_into()?)
    }
}
