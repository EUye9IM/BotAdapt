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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_config_defaults_enabled_to_true() {
        let toml_str = r#"
            name = "test"
            path = "./test.wasm"
        "#;
        let cfg: PluginConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.name, "test");
        assert_eq!(cfg.path, "./test.wasm");
        assert!(cfg.enabled);
    }

    #[test]
    fn plugin_config_explicit_disabled() {
        let toml_str = r#"
            name = "test"
            path = "./test.wasm"
            enabled = false
        "#;
        let cfg: PluginConfig = toml::from_str(toml_str).unwrap();
        assert!(!cfg.enabled);
    }

    #[test]
    fn binding_config_defaults() {
        let toml_str = r#"
            plugins = ["hello"]
        "#;
        let cfg: BindingConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.botid, "*");
        assert_eq!(cfg.target_type, "*");
        assert_eq!(cfg.target_id, "*");
        assert!(cfg.enabled);
        assert_eq!(cfg.plugins, vec!["hello"]);
    }

    #[test]
    fn full_config_with_plugins() {
        let toml_str = r#"
            [core]
            log_level = "debug"

            [[bots]]
            id = "bot1"
            platform = "stdio"

            [[plugins]]
            name = "hello"
            path = "./hello.wasm"

            [[bindings]]
            plugins = ["*"]
        "#;
        let mut value: toml::Value = toml::from_str(toml_str).unwrap();
        parser::expand_value(&mut value);
        let config: Config = value.try_into().unwrap();
        assert_eq!(config.core.log_level, "debug");
        assert_eq!(config.bots.len(), 1);
        assert_eq!(config.plugins.len(), 1);
        assert_eq!(config.plugins[0].name, "hello");
        assert_eq!(config.bindings.len(), 1);
    }
}
