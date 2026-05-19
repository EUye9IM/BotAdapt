# Phase 3 — WASM 插件系统 实施计划

## 3.1 SDK 层 (`botadapt-plugin-sdk`)

- [ ] **`prelude.rs`** — 统一重导出 `Event`, `MessageEvent`, `MessageContent`, `MessageTarget`, `Action`，方便插件 `use botadapt_plugin_sdk::prelude::*`
- [ ] **`host_calls.rs`** — 声明 host 函数 (`extern "C"`)，用于插件侧调用:
  - `host_log(level: i32, ptr: i32, len: i32)`
  - `host_send_message(target_ptr: i32, target_len: i32, content_ptr: i32, content_len: i32) -> i32`
  - `host_get_config(ptr: i32, len: i32) -> i32`
- [ ] **`lib.rs`** — 导出 `prelude`, `host_calls`
- [ ] **`types/event.rs`** — 补齐字段，与 core 对齐:
  - `Event.source_adapter: Option<String>`
  - `Event.id: String` → 保持 String（WASM 侧无 uuid 依赖）
- [ ] **`types/target.rs`** — 补齐 `MessageTarget.adapter_instance: Option<String>`
- [ ] **ABI 导出符号约定** — 插件需导出:
  - `plugin_version() -> i32` — ABI 版本号
  - `plugin_init(ptr: i32, len: i32) -> i32` — 接收 JSON 配置，返回 0 表示成功
  - `plugin_handle_event(ptr: i32, len: i32) -> i32` — 接收 Event JSON，返回 Action JSON
  - `plugin_destroy()` — 卸载清理

## 3.2 Core WASM 运行时 (`botadapt-core`)

- [ ] **`Cargo.toml`** — 添加 `wasmtime` 依赖 (默认 features，包括 cranelift JIT)
- [ ] **`plugin/wasm/mod.rs`** — 模块入口，重新导出子模块
- [ ] **`plugin/wasm/runtime.rs`** — Engine 管理:
  - `pub struct WasmRuntime { engine: Engine }` — 全局共享 (Send + Sync)
  - `fn new() -> Self` — 创建 Engine
  - `fn engine(&self) -> &Engine`
- [ ] **`plugin/wasm/instance.rs`** — PluginInstance:
  - `pub struct PluginInstance { name: String, store: Mutex<Store<PluginData>>, handle_event: TypedFunc<(i32, i32), i32> }`
  - `fn load(engine: &Engine, path: &Path) -> Result<Self>` — 编译 .wasm → 实例化 → 绑定 host 函数
  - `fn call_handle_event(&self, event_json: &[u8]) -> Result<Vec<u8>>` — 写入 memory + 调用函数 + 读取返回值
  - `Store<PluginData>` 中 `PluginData` 保存 adapter 引用、logger 等
- [ ] **`plugin/wasm/abi.rs`** — ABI 常量:
  - 导出符号名常量 (`HANDLE_EVENT`, `INIT`, `VERSION`, `DESTROY`)
  - 内存 alloc/free 符号名 (如 `_initialize`, `cabi_*` 或手动管理)
- [ ] **`plugin/wasm/host_fns.rs`** — Host 函数实现:
  - `host_log` — 接收 (level, ptr, len)，从 wasm memory 读字符串，通过 tracing 输出
  - `host_send_message` — 接收 target + content ptr/lens，反序列化 JSON，通过已有的 AdapterRegistry 发送消息
  - `host_get_config` — 回调传入的 config 到 wasm memory
- [ ] **`plugin/mod.rs`** — 导出 `pub mod wasm`
- [ ] **`plugin/manager.rs`** — 集成 WASM 插件:
  - 添加 `load_wasm(&mut self, config: &PluginConfig) -> Result<()>` — 编译 .wasm → 创建 PluginInstance → 调 init → 注册
  - 添加 `unload_wasm(&mut self, name: &str) -> Result<()>` — 调 destroy → 移除
  - `dispatch_parallel` 已支持 `dyn Plugin`，需做一个 `WasmPlugin` 包装实现 `Plugin` trait（内部委托给 PluginInstance）

## 3.3 Dice 插件 (`plugins/dice`)

- [ ] **`plugins/dice/Cargo.toml`** — wasm32-wasip1 target，依赖 botadapt-plugin-sdk
- [ ] **`plugins/dice/src/lib.rs`** — 实现:
  - 导出 `plugin_version`, `plugin_init`, `plugin_handle_event`, `plugin_destroy`
  - `plugin_handle_event`: 解析 Event JSON → 提取消息文本 → 正则匹配 `(\d+)d(\d+)`
  - 骰子逻辑: `ndm` = n 个 1..m 的随机数，生成如 `2d6 = 4+2 = 6` 格式
  - 构造 `Action::SendMessage` 返回 JSON
  - 随机数: 使用 wasi `random_get` 或简单的 LCG 算法
- [ ] **编译命令** — `cargo build --target wasm32-wasip1 --release`

## 3.4 配置集成

- [ ] **`botadapt.toml`** — 新增 dice 插件声明，channel 绑定:
  ```toml
  [[plugins]]
  name = "dice"
  path = "./plugins/dice.wasm"
  enabled = true

  # 在已有 channel 中追加 dice 插件（例如 group:* 绑定 builtin + dice）
  [[adapters.channels]]
  channel_id = "group:*"
  plugins = ["builtin", "dice"]
  ```
- [ ] **`botadapt-cli/src/main.rs`** — 启动时根据 `config.plugins` 加载 WASM 插件
