use std::path::Path;

use super::Config;

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut value: toml::Value = toml::from_str(&content)?;
        expand_value(&mut value);
        let expanded = toml::to_string(&value).map_err(|e| anyhow::anyhow!(e))?;
        let config: Config = toml::from_str(&expanded)?;
        Ok(config)
    }
}

fn expand_value(value: &mut toml::Value) {
    match value {
        toml::Value::String(s) => {
            *s = expand_env_vars(s);
        }
        toml::Value::Table(t) => {
            for (_, v) in t.iter_mut() {
                expand_value(v);
            }
        }
        toml::Value::Array(a) => {
            for v in a.iter_mut() {
                expand_value(v);
            }
        }
        _ => {}
    }
}

fn expand_env_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut var_expr = String::new();
            while let Some(&c) = chars.peek() {
                if c == '}' {
                    chars.next(); // consume '}'
                    break;
                }
                var_expr.push(c);
                chars.next();
            }
            let resolved = resolve_var(&var_expr);
            result.push_str(&resolved);
        } else {
            result.push(ch);
        }
    }

    result
}

fn resolve_var(expr: &str) -> String {
    if let Some((name, default)) = expr.split_once(":-") {
        std::env::var(name).unwrap_or_else(|_| default.to_string())
    } else {
        std::env::var(expr).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_no_var() {
        assert_eq!(expand_env_vars("hello world"), "hello world");
    }

    #[test]
    fn expand_var_exists() {
        std::env::set_var("TEST_EXPAND_A", "value123");
        assert_eq!(
            expand_env_vars("prefix_${TEST_EXPAND_A}_suffix"),
            "prefix_value123_suffix"
        );
    }

    #[test]
    fn expand_var_missing() {
        std::env::remove_var("TEST_NONEXIST");
        assert_eq!(expand_env_vars("${TEST_NONEXIST}"), "");
    }

    #[test]
    fn expand_var_default() {
        std::env::remove_var("TEST_NONEXIST2");
        assert_eq!(expand_env_vars("${TEST_NONEXIST2:-fallback}"), "fallback");
    }

    #[test]
    fn expand_var_default_with_existing() {
        std::env::set_var("TEST_EXPAND_B", "real");
        assert_eq!(expand_env_vars("${TEST_EXPAND_B:-fallback}"), "real");
    }

    #[test]
    fn expand_multiple_vars() {
        std::env::set_var("TEST_A", "a");
        std::env::set_var("TEST_B", "b");
        assert_eq!(expand_env_vars("${TEST_A}:${TEST_B}"), "a:b");
    }

    #[test]
    fn toml_value_expand_recursive() {
        let toml_str = r#"
            app_id = "${TEST_A}"
            [nested]
            key = "val_${TEST_B:-x}"
        "#;
        std::env::set_var("TEST_A", "app123");
        std::env::remove_var("TEST_B");
        let mut value: toml::Value = toml::from_str(toml_str).unwrap();
        expand_value(&mut value);
        let expanded = toml::to_string(&value).unwrap();
        assert!(expanded.contains("app_id = \"app123\""));
        assert!(expanded.contains("key = \"val_x\""));
    }
}
