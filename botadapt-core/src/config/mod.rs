pub mod parser;

use serde::Deserialize;
use serde_inline_default::serde_inline_default;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub adapters: Vec<AdapterConfig>,
    #[serde(default)]
    pub plugins: Vec<PluginConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoreConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
        }
    }
}

fn default_log_level() -> String {
    "info".into()
}
#[serde_inline_default]
#[derive(Debug, Clone, Deserialize)]
pub struct AdapterConfig {
    #[serde(rename = "type")]
    pub adapter_type: String,
    #[serde_inline_default(true)]
    pub enabled: bool,
    pub name: String,
    #[serde(default)]
    pub config: toml::Table,
    #[serde(default)]
    pub channels: Vec<ChannelEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelEntry {
    pub channel_id: String,
    #[serde(default)]
    pub plugins: Vec<String>,
}
#[serde_inline_default]
#[derive(Debug, Clone, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub path: String,
    #[serde_inline_default(true)]
    pub enabled: bool,
}
