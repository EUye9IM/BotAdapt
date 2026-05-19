use wasmtime::{Caller, Engine, Linker};
use wasmtime_wasi::preview1::WasiP1Ctx;
use wasmtime_wasi::WasiCtxBuilder;

use crate::error::{Error, Result};

pub struct PluginData {
    pub config: serde_json::Value,
    pub wasi: WasiP1Ctx,
}

impl PluginData {
    pub fn new(config: serde_json::Value) -> Self {
        Self {
            config,
            wasi: WasiCtxBuilder::new().build_p1(),
        }
    }
}

pub fn create_linker(engine: &Engine) -> Result<Linker<PluginData>> {
    let mut linker = Linker::new(engine);
    wasmtime_wasi::preview1::add_to_linker_sync(
        &mut linker,
        |data: &mut PluginData| &mut data.wasi,
    )
    .map_err(|e| Error::Config(format!("注册 WASI 失败: {}", e)))?;
    add_host_functions(&mut linker)?;
    Ok(linker)
}

fn add_host_functions(linker: &mut Linker<PluginData>) -> Result<()> {
    linker
        .func_wrap("env", "host_log", |mut caller: Caller<'_, PluginData>, level: i32, ptr: i32, len: i32| {
            if ptr == 0 || len <= 0 {
                return;
            }
            let mem = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => return,
            };
            let mut buf = vec![0u8; len as usize];
            if mem.read(&caller, ptr as usize, &mut buf).is_err() {
                return;
            }
            let msg = String::from_utf8_lossy(&buf);
            match level {
                1 => tracing::error!("[wasm] {}", msg),
                2 => tracing::warn!("[wasm] {}", msg),
                3 => tracing::info!("[wasm] {}", msg),
                4 => tracing::debug!("[wasm] {}", msg),
                _ => tracing::trace!("[wasm] {}", msg),
            }
        })
        .map_err(|e| Error::Config(format!("注册 host_log 失败: {}", e)))?;

    linker
        .func_wrap(
            "env",
            "host_get_config",
            |mut caller: Caller<'_, PluginData>, ptr: i32, max_len: i32| -> i32 {
                let config = &caller.data().config;
                let json = serde_json::to_string(config).unwrap_or_default();
                let bytes = json.as_bytes();
                let write_len = bytes.len().min(max_len as usize);

                let mem = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(m)) => m,
                    _ => return 0,
                };
                if mem
                    .write(&mut caller, ptr as usize, &bytes[..write_len])
                    .is_err()
                {
                    return 0;
                }
                write_len as i32
            },
        )
        .map_err(|e| Error::Config(format!("注册 host_get_config 失败: {}", e)))?;

    Ok(())
}
