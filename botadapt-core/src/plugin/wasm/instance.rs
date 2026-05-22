use std::sync::Mutex;

use wasmtime::{Engine, Memory, Module, Store, TypedFunc};

use crate::error::{Error, Result};

use super::host_fns::{create_linker, PluginData};

pub struct PluginInstance {
    store: Mutex<Store<PluginData>>,
    handle_event: TypedFunc<(i32, i32), i64>,
    memory: Memory,
}

impl PluginInstance {
    pub fn load(
        engine: Engine,
        wasm_bytes: &[u8],
        config: serde_json::Value,
    ) -> Result<Self> {
        let module = Module::from_binary(&engine, wasm_bytes)
            .map_err(|e| Error::Config(e.to_string()))?;

        let mut store = Store::new(&engine, PluginData::new(config));
        let linker = create_linker(&engine)?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| Error::Config(format!("WASM 实例化失败: {}", e)))?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| Error::Config("WASM 模块缺少 memory 导出".into()))?;

        let handle_event = instance
            .get_typed_func::<(i32, i32), i64>(&mut store, "plugin_handle_event")
            .map_err(|e| Error::Config(format!("WASM 模块缺少 plugin_handle_event 导出: {}", e)))?;

        Ok(Self {
            store: Mutex::new(store),
            handle_event,
            memory,
        })
    }

    pub fn call_handle_event(&self, event_json: &[u8]) -> Result<Vec<u8>> {
        let mut store = self.store.lock().map_err(|e| {
            Error::Plugin(format!("WASM store lock 失败: {}", e))
        })?;

        let data_len = event_json.len();

        // 在 wasm linear memory 末尾写入 event JSON
        let current_pages = self.memory.size(&*store);
        let current_bytes = (current_pages as usize) * 0x10000;
        let mut offset = current_bytes as u64;
        let needed_pages = ((current_bytes + data_len + 0xFFFF) / 0x10000) as u64;

        // 如果内存不够，grow
        if needed_pages > current_pages {
            self.memory
                .grow(&mut *store, needed_pages - current_pages)
                .map_err(|e| Error::Plugin(format!("WASM memory grow 失败: {}", e)))?;
            offset = (current_pages as u64) * 0x10000;
        }

        self.memory
            .write(&mut *store, offset as usize, event_json)
            .map_err(|e| Error::Plugin(format!("写入 WASM memory 失败: {}", e)))?;

        let result = self
            .handle_event
            .call(&mut *store, (offset as i32, data_len as i32))
            .map_err(|e| Error::Plugin(format!("调用 plugin_handle_event 失败: {}", e)))?;

        let result_ptr = (result >> 32) as i32;
        let result_len = (result as i32 & 0x7FFFFFFFi32)
            .max(0);

        if result_ptr == 0 || result_len <= 0 {
            return Ok(vec![]);
        }

        let mut buf = vec![0u8; result_len as usize];
        self.memory
            .read(&*store, result_ptr as usize, &mut buf)
            .map_err(|e| Error::Plugin(format!("读取 WASM memory 返回值失败: {}", e)))?;

        Ok(buf)
    }
}

use std::sync::Arc;

use async_trait::async_trait;

use crate::event::Event;
use crate::plugin::{Action, Plugin};

pub struct WasmPlugin {
    name: String,
    instance: Arc<PluginInstance>,
}

impl WasmPlugin {
    pub fn new(name: String, instance: Arc<PluginInstance>) -> Self {
        Self { name, instance }
    }
}

#[async_trait]
impl Plugin for WasmPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn handle_event(&self, event: Event) -> Result<Vec<Action>> {
        let event_json = serde_json::to_vec(&event)?;
        let result_json = self.instance.call_handle_event(&event_json)?;
        if result_json.is_empty() {
            return Ok(vec![]);
        }
        let actions: Vec<Action> = serde_json::from_slice(&result_json)?;
        Ok(actions)
    }
}
