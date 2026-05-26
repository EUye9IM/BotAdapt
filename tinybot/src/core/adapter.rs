use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

pub struct AdapterRegistry {
    adapters: HashMap<String, Arc<dyn Adapter>>,
}

#[async_trait]
pub trait Adapter: Send + Sync {
    /// 启动适配器事件循环。
    ///
    /// 通过 `emit` 回调投递事件；回调内部负责设置 `source_adapter` 及发送。
    async fn start(
        &self,
        emit: Box<dyn Fn(super::events::AdapterEvent) + Send + Sync + 'static>,
        shutdown: tokio_util::sync::CancellationToken,
    ) -> anyhow::Result<()>;
    async fn send_message(&self, message: &super::events::MessageEvent) -> anyhow::Result<()>;
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, adapter: Arc<dyn Adapter>) {
        // todo 冲突检测
        self.adapters.insert(name.to_owned(), adapter);
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, Arc<dyn Adapter>> {
        self.adapters.iter()
    }
    pub fn get(&self, name: &str) -> Option<Arc<dyn Adapter>> {
        self.adapters.get(name).cloned()
    }
}
