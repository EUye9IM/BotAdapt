mod parser;
use std::path::Path;

use serde::Deserialize;
use serde_inline_default::serde_inline_default;
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub bots: Vec<BotConfig>,
    #[serde(default)]
    pub plugins: Vec<PluginConfig>,
    #[serde(default)]
    pub bindings: Vec<BindingConfig>,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut value: toml::Value = toml::from_str(&content)?;
        parser::expand_value(&mut value);
        let config: Config = value
            .try_into()
            .map_err(|e| anyhow::anyhow!("parse config failed: {}", e))?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde_inline_default]
pub struct CoreConfig {
    #[serde_inline_default("info".into())]
    pub log_level: String,
}

#[serde_inline_default]
#[derive(Debug, Clone, Deserialize)]
pub struct BotConfig {
    pub id: String,
    pub platform: String,
    #[serde_inline_default(true)]
    pub enabled: bool,
    #[serde(default)]
    pub config: toml::Table,
}

#[serde_inline_default]
#[derive(Debug, Clone, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub path: String,
    #[serde_inline_default(true)]
    pub enabled: bool,
}

#[serde_inline_default]
#[derive(Debug, Clone, Deserialize)]
pub struct BindingConfig {
    #[serde_inline_default("*".into())]
    pub botid: String,
    #[serde_inline_default("*".into())]
    pub target_type: String,
    #[serde_inline_default("*".into())]
    pub target_id: String,
    #[serde_inline_default(true)]
    pub enabled: bool,
    #[serde(default)]
    pub plugins: Vec<String>,
}
