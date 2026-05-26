pub fn expand_value(value: &mut toml::Value) {
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
        temp_env::with_var("TEST_EXPAND_A", Some("value123"), || {
            assert_eq!(
                expand_env_vars("prefix_${TEST_EXPAND_A}_suffix"),
                "prefix_value123_suffix"
            );
        })
    }

    #[test]
    fn expand_var_missing() {
        temp_env::with_var_unset("TEST_NONEXIST", || {
            assert_eq!(expand_env_vars("${TEST_NONEXIST}"), "");
        })
    }

    #[test]
    fn expand_var_default() {
        temp_env::with_var_unset("TEST_NONEXIST", || {
            assert_eq!(expand_env_vars("${TEST_NONEXIST:-fallback}"), "fallback");
        })
    }

    #[test]
    fn expand_var_default_with_existing() {
        temp_env::with_var("TEST_EXPAND_B", Some("real"), || {
            assert_eq!(expand_env_vars("${TEST_EXPAND_B:-fallback}"), "real");
        })
    }

    #[test]
    fn expand_multiple_vars() {
        temp_env::with_vars(vec![("TEST_A", Some("a")), ("TEST_B", Some("b"))], || {
            assert_eq!(expand_env_vars("${TEST_A}:${TEST_B}"), "a:b");
        })
    }

    #[test]
    fn toml_value_expand_recursive() {
        let toml_str = r#"
            app_id = "${TEST_A}"
            [nested]
            key = "val_${TEST_B:-x}"
        "#;
        temp_env::with_vars(vec![("TEST_A", Some("app123")), ("TEST_B", None)], || {
            let mut value: toml::Value = toml::from_str(toml_str).unwrap();
            expand_value(&mut value);
            let expanded = toml::to_string(&value).unwrap();
            assert!(expanded.contains("app_id = \"app123\""));
            assert!(expanded.contains("key = \"val_x\""));
        });
    }
}
