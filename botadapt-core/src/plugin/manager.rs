use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use super::wasm::{PluginInstance, WasmPlugin};
use super::{Action, Plugin};
use crate::error::{Error, Result};
use crate::event::Event;
use tracing::Instrument;
use wasmtime::Engine;

pub struct PluginManager {
    engine: Engine,
    plugins: HashMap<String, Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            engine: Engine::default(),
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

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn register_wasm_instance(&mut self, name: &str, instance: Arc<PluginInstance>) {
        let plugin = Box::new(WasmPlugin::new(name.to_string(), instance));
        self.register(plugin);
    }

    pub async fn load_wasm(
        &mut self,
        name: &str,
        path: &Path,
        config: serde_json::Value,
    ) -> Result<()> {
        let wasm_bytes = tokio::fs::read(path).await?;
        let engine = self.engine.clone();
        let plugin_name = name.to_string();
        let instance = tokio::task::spawn_blocking(move || {
            PluginInstance::load(engine, &wasm_bytes, config)
        })
        .await
        .map_err(|e| Error::Plugin(format!("WASM spawn_blocking 失败: {}", e)))??;
        let plugin = Box::new(WasmPlugin::new(plugin_name, Arc::new(instance)));
        self.register(plugin);
        Ok(())
    }

    pub async fn dispatch_parallel(&self, event: &Event, names: &[String]) -> Vec<Action> {
        let mut tasks = Vec::new();

        for name in names {
            if let Some(plugin) = self.plugins.get(name) {
                let ev = event.clone();
                let span = tracing::info_span!("plugin", plugin = %name);
                tasks.push(plugin.handle_event(ev).instrument(span));
            }
        }

        let results = futures::future::join_all(tasks).await;
        let mut all_actions = Vec::new();

        for (i, result) in results.into_iter().enumerate() {
            let plugin_name = names.get(i).map(|s| s.as_str()).unwrap_or("unknown");
            match result {
                Ok(actions) => {
                    tracing::trace!("插件 {} 返回 {} 个 Action", plugin_name, actions.len());
                    all_actions.extend(actions);
                }
                Err(e) => tracing::error!("插件 {} 处理事件失败: {}", plugin_name, e),
            }
        }

        all_actions
    }
}
