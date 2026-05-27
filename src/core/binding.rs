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
