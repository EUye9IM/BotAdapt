use std::path::Path;

use crate::error::Result;

use super::Config;

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
