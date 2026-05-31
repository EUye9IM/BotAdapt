use std::collections::HashSet;

use glob_match;

use crate::core::config;

struct Binding {
    pub bis: String,
    pub target_type: String,
    pub target_id: String,
    pub plugins: Vec<String>,
}

pub struct Bindings {
    bindings: Vec<Binding>,
}
impl Bindings {
    pub fn new(cfg: Vec<config::BindingConfig>) -> Self {
        let bindings = cfg
            .into_iter()
            .filter(|c| c.enabled)
            .map(|c| Binding {
                bis: c.botid,
                target_type: c.target_type,
                target_id: c.target_id,
                plugins: c.plugins,
            })
            .collect();
        Bindings { bindings }
    }
    pub fn get_plugin_list(
        &self,
        bid: &str,
        ttype: &str,
        tid: &str,
        plugins: HashSet<String>,
    ) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for binding in &self.bindings {
            if !glob_match::glob_match(&binding.bis, bid)
                || !glob_match::glob_match(&binding.target_type, ttype)
                || !glob_match::glob_match(&binding.target_id, tid)
            {
                continue;
            }
            for plugin_pattern in &binding.plugins {
                for plugin in &plugins {
                    if glob_match::glob_match(plugin_pattern, plugin) && seen.insert(plugin.clone())
                    {
                        result.push(plugin.clone());
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::BindingConfig;

    fn cfg(botid: &str, target_type: &str, target_id: &str, plugins: Vec<&str>) -> BindingConfig {
        BindingConfig {
            botid: botid.to_string(),
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
            enabled: true,
            plugins: plugins.into_iter().map(String::from).collect(),
        }
    }

    fn plugins(list: &[&str]) -> HashSet<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn empty_bindings_returns_empty() {
        let b = Bindings::new(vec![]);
        let result = b.get_plugin_list("bot1", "group", "123", plugins(&[]));
        assert!(result.is_empty());
    }

    #[test]
    fn exact_match_single_binding() {
        let b = Bindings::new(vec![cfg("bot1", "group", "123", vec!["hello"])]);
        let result = b.get_plugin_list("bot1", "group", "123", plugins(&["hello"]));
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn wildcard_match_botid() {
        let b = Bindings::new(vec![cfg("*", "group", "123", vec!["hello"])]);
        let result = b.get_plugin_list("bot2", "group", "123", plugins(&["hello"]));
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn wildcard_match_target_type() {
        let b = Bindings::new(vec![cfg("bot1", "*", "123", vec!["hello"])]);
        let result = b.get_plugin_list("bot1", "group", "123", plugins(&["hello"]));
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn no_match_returns_empty() {
        let b = Bindings::new(vec![cfg("bot1", "group", "123", vec!["hello"])]);
        let result = b.get_plugin_list("bot1", "group", "999", plugins(&["hello"]));
        assert!(result.is_empty());
    }

    #[test]
    fn wildcard_plugin_pattern_matches_any() {
        let b = Bindings::new(vec![cfg("bot1", "group", "123", vec!["*"])]);
        let mut result =
            b.get_plugin_list("bot1", "group", "123", plugins(&["hello", "dice"]));
        result.sort();
        assert_eq!(result, vec!["dice", "hello"]);
    }

    #[test]
    fn plugin_dedup() {
        let b = Bindings::new(vec![
            cfg("bot1", "group", "123", vec!["hello"]),
            cfg("bot1", "group", "123", vec!["hello", "dice"]),
        ]);
        let mut result =
            b.get_plugin_list("bot1", "group", "123", plugins(&["hello", "dice"]));
        result.sort();
        assert_eq!(result, vec!["dice", "hello"]);
    }

    #[test]
    fn disabled_binding_ignored() {
        let mut disabled = cfg("bot1", "group", "123", vec!["hello"]);
        disabled.enabled = false;
        let b = Bindings::new(vec![disabled]);
        let result = b.get_plugin_list("bot1", "group", "123", plugins(&["hello"]));
        assert!(result.is_empty());
    }

    #[test]
    fn group_prefix_glob() {
        let b = Bindings::new(vec![cfg("bot1", "group:*", "123", vec!["hello"])]);
        let result = b.get_plugin_list("bot1", "group:456", "123", plugins(&["hello"]));
        assert_eq!(result, vec!["hello"]);
    }
}
