use std::collections::hash_map::Iter;
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

    pub fn register(&mut self, name: &str, adapter: Arc<dyn Adapter>) {
        self.adapters.insert(name.to_owned(), adapter);
    }

    pub fn iter(&self) -> Iter<'_, String, Arc<dyn Adapter>> {
        self.adapters.iter()
    }
    pub fn get(&self, name: &str) -> Option<Arc<dyn Adapter>> {
        self.adapters.get(name).cloned()
    }
}
