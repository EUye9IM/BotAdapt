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

    pub fn iter(&self) -> Iter<String, Arc<dyn Adapter>> {
        self.adapters.iter()
    }
    pub fn get(&self, name: &str) -> Option<Arc<dyn Adapter>> {
        self.adapters.get(name).cloned()
    }

    // pub fn ids(&self) -> impl Iterator<Item = &str> {
    //     self.adapters.keys().map(|s| s.as_str())
    // }

    // pub fn find_by_platform(&self, platform: &str) -> Option<Arc<dyn Adapter>> {
    //     self.adapters
    //         .values()
    //         .find(|a| a.platform_id() == platform)
    //         .cloned()
    // }
}
