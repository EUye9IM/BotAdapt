use std::collections::HashMap;

use super::{Action, Plugin};
use crate::event::Event;

pub struct PluginManager {
    plugins: HashMap<String, Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let name = plugin.name().to_string();
        self.plugins.insert(name, plugin);
    }

    pub fn unregister(&mut self, name: &str) -> Option<Box<dyn Plugin>> {
        self.plugins.remove(name)
    }

    pub fn get(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins.get(name).map(|p| p.as_ref())
    }

    /// 并行调用指定插件列表。各插件独立执行，互不依赖。
    /// 一个插件失败不影响其他插件。
    pub async fn dispatch_parallel(&self, event: &Event, names: &[String]) -> Vec<Action> {
        // Phase 1: 暂用并发 join (futures::future::join_all)，
        // 后续根据 Wasm Store 并发模型决定是否改用 tokio::spawn
        let mut tasks = Vec::new();

        for name in names {
            if let Some(plugin) = self.plugins.get(name) {
                let ev = event.clone();
                tasks.push(plugin.handle_event(ev));
            }
        }

        let results = futures::future::join_all(tasks).await;
        let mut all_actions = Vec::new();

        for result in results {
            match result {
                Ok(actions) => all_actions.extend(actions),
                Err(e) => tracing::error!("插件处理事件失败: {}", e),
            }
        }

        all_actions
    }
}
