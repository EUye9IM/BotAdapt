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
        let platform = adapter.platform_id().to_string();
        self.adapters.insert(platform, adapter);
    }

    pub fn get(&self, platform: &str) -> Option<Arc<dyn Adapter>> {
        self.adapters.get(platform).cloned()
    }

    pub fn platforms(&self) -> impl Iterator<Item = &str> {
        self.adapters.keys().map(|s| s.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }
}
