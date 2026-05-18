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
        let id = adapter.instance_id();
        self.adapters.insert(id, adapter);
    }

    pub fn get(&self, instance_id: &str) -> Option<Arc<dyn Adapter>> {
        self.adapters.get(instance_id).cloned()
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.adapters.keys().map(|s| s.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }
}
