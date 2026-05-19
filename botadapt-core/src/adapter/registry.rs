use std::collections::HashMap;
use std::sync::Arc;

use super::Adapter;

pub struct AdapterRegistry {
    adapters: HashMap<String, Arc<dyn Adapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    pub fn register(&mut self, adapter: Arc<dyn Adapter>) {
        let name = adapter.name();
        self.adapters.insert(name, adapter);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Adapter>> {
        self.adapters.get(name).cloned()
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.adapters.keys().map(|s| s.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }

    pub fn find_by_platform(&self, platform: &str) -> Option<Arc<dyn Adapter>> {
        self.adapters
            .values()
            .find(|a| a.platform_id() == platform)
            .cloned()
    }
}
