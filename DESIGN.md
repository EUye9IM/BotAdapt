# BotAdapt — 设计文档

## 1. 概述

**BotAdapt** 是一个通用异步 Bot 框架，基于 Rust + Tokio 构建。核心设计理念是**平台无关的事件驱动 + 插件化**——一次编写插件逻辑，通过配置挂载到任意消息平台。

### 1.1 核心目标

| 目标 | 说明 |
|------|------|
| 平台抽象 | 统一事件模型屏蔽 QQ / Discord / Telegram 等差异 |
| 插件化 | 首选 Wasm 插件，支持热加载和安全隔离 |
| 配置驱动 | 启动时通过配置文件声明加载哪些 adapter 和 plugin |
| 异步高效 | 基于 Tokio，最小化上下文切换开销 |

---

## 2. 架构总览

```
┌────────────────────────────────────────────────┐
│                   Plugins (Wasm .wasm)           │
│   echo.wasm  ·  admin.wasm  ·  custom.wasm      │
└──────────────────────┬─────────────────────────┘
                       │ wasmtime 调用 (并行)
┌──────────────────────▼─────────────────────────┐
│              Plugin Runtime (Wasmtime)           │
│    沙箱执行 · ABI 校验 · 资源限制 · 热加载        │
└──────────────────────┬─────────────────────────┘
                       │ PluginHandle (泛化接口)
┌──────────────────────▼─────────────────────────┐
│                botadapt-core                     │
│  ┌──────────┐ ┌───────────────┐ ┌────────────┐ │
│  │ Event Bus│ │Channel Binding│ │AdapterManager│ │
│  │ (广播)   │ │  (ch→plugins) │ │(注册/生命周期)│ │
│  └──────────┘ └───────────────┘ └────────────┘ │
└──────┬────────────────────────────────┬─────────┘
       │ 统一的 Adapter trait            │
  ┌────▼────┐                    ┌──────▼──────┐
  │ QQ       │                    │  (未来)      │
  │ Adapter  │                    │  Discord     │
  │          │                    │  Adapter     │
  └──────────┘                    └─────────────┘
```

### 分发模型

插件之间**无依赖关系，并行执行**。每个事件到达后，core 根据事件的 `channel_id` 查表找到绑定的插件列表，并发调用，收集所有插件返回的 Action 后统一执行。

### 数据流

```
Platform (QQ) ──WebSocket──► QQ Adapter ──Event──► Event Bus
                                                      │
                                         ┌────────────▼────────────┐
                                         │  查 Channel Binding 表   │
                                         │  (event.channel_id →     │
                                         │   plugins 列表)          │
                                         └────────────┬────────────┘
                                                      │
                                         ┌────────────┼────────────┐
                                         ▼            ▼            ▼
                                    Plugin A    Plugin B    Plugin C
                                    (并行执行，互不依赖)
                                         │            │            │
                                         └─────┬──────┴─────┬──────┘
                                               ▼            ▼
                                      收集所有 Action，逐个执行
                                               │
                                          Adapter ──HTTP──► Platform API

---

## 3. 模块划分

```
botadapt/
├── Cargo.toml                      # workspace
├── botadapt-core/                  # [核心] 框架本体
├── botadapt-qq/                    # [Adapter] QQ 官方 API
├── botadapt-plugin-sdk/            # [SDK] 插件开发库 (no_std 兼容)
├── botadapt-cli/                   # [入口] CLI 二进制
├── plugins/                        # 插件项目 + .wasm 产物
```

### 3.1 `botadapt-core`

框架核心，零平台依赖。

```
src/
├── lib.rs
├── config/
│   ├── mod.rs              # Config 结构体
│   ├── parser.rs           # 配置解析 (toml)
│   └── watcher.rs          # 配置热重载
├── event/
│   ├── mod.rs              # Event / MessageEvent 等统一类型
│   └── bus.rs              # 广播 + 订阅
├── adapter/
│   ├── mod.rs              # Adapter trait
│   └── registry.rs         # Adapter 注册与查找 (按 name)
├── plugin/
│   ├── mod.rs              # Plugin trait + Action
│   ├── wasm/
│   │   ├── mod.rs          # 模块入口
│   │   ├── instance.rs     # PluginInstance: 加载 & 调用 + WasmPlugin 包装
│   │   └── host_fns.rs     # host 函数实现 (log, get_config)
│   ├── native.rs           # BuiltinPlugin 实现
│   └── manager.rs          # 插件加载/卸载 + 并行分发
├── binding.rs              # Channel → PluginList 绑定表 (按 adapter 分组)
└── error.rs                # 统一错误类型
```

**核心 Trait**:

```rust
#[async_trait]
pub trait Adapter: Send + Sync {
    /// self_name 为适配器在注册表中的名称（如 "default"），
    /// 用于事件溯源（写入 Event.source_adapter）及日志标识。
    async fn start(&self, self_name: String, tx: mpsc::Sender<Event>, shutdown: CancellationToken) -> Result<()>;
    async fn send_message(&self, target: &MessageTarget, content: &MessageContent) -> Result<()>;
}

/// 适配器注册表按 name 查找 Adapter 实例
/// 当插件调用 send_message 时，host 通过 MessageTarget.adapter_instance
/// 在 AdapterRegistry 中找到对应 adapter，调用其 send_message。
```

### 3.2 `botadapt-qq`

腾讯官方 QQ 机器人 API 的具体实现。

```
src/
├── lib.rs
├── adapter.rs              # 实现 Adapter trait
├── config.rs               # QQ 专属配置 (app_id, token, 等)
├── ws/
│   ├── client.rs           # WebSocket 连接管理 (tokio-tungstenite)
│   └── heartbeat.rs        # 心跳
├── api/
│   ├── mod.rs              # QQ HTTP API 封装
│   ├── message.rs          # 消息发送 API
│   └── types.rs            # API 响应类型
└── event/
    └── converter.rs        # QQ 原生事件 → 统一 Event 转换
```

通信方式：**WebSocket（官方事件推送）+ HTTP（主动调用 API）**。

### 3.3 `botadapt-plugin-sdk`

给插件开发者使用的库。编译目标为 `wasm32-wasip1`。

```rust
// 用户插件代码示例（dice 插件实际实现）
use botadapt_plugin_sdk::prelude::*;

// 导出 ABI 符号
#[no_mangle] pub extern "C" fn plugin_version() -> i32 { 1 }

#[no_mangle]
pub extern "C" fn plugin_handle_event(event_ptr: i32, event_len: i32) -> i64 {
    // 1. 从 wasm memory 读取 Event JSON
    // 2. 解析消息文本，匹配命令（如 ndm）
    // 3. 构造 Action JSON，写回 wasm memory
    // 4. 返回 packed (ptr << 32 | len)
    todo!()
}
```

```
src/
├── lib.rs
├── types/
│   ├── event.rs            # Event (serde 序列化)
│   ├── action.rs           # Action: SendMessage / Noop
│   └── target.rs           # MessageTarget
├── host_calls.rs           # extern "C" host 函数声明
├── prelude.rs              # 统一重导出
└── lib.rs
```

**Action 类型**（纯副作用，无控制流语义）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    SendMessage {
        target: MessageTarget,
        content: MessageContent,
    },
    Noop,
}
```

### 3.4 `botadapt-cli`

```rust
// main.rs
#[tokio::main]
async fn main() {
    let config = load_config(std::env::args())?;
    let app = BotAdapt::new(config).await?;
    app.run().await?;  // 启动所有 adapter + plugin 循环
}
```

---

## 4. 事件模型

统一事件是框架的**核心抽象**。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub channel_id: String,         // channel 标识，格式: "{type}:{id}"
                                    // 示例: "group:123456", "c2c:USER_OPENID"
    pub platform: String,           // "qq" (平台标识，便于插件做差异逻辑)
    pub source_adapter: Option<String>, // 产生事件的适配器 name
    pub timestamp: i64,
    pub kind: EventKind,
}
```

**channel_id 的作用**：Core 根据 `event.channel_id` 查找 Channel Binding 表中该 channel 关联的插件列表，然后并行调用。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
    Message(MessageEvent),
    Notice(NoticeEvent),
    Request(RequestEvent),
    Meta(MetaEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEvent {
    pub user_id: String,
    pub group_id: Option<String>,
    pub channel_id: Option<String>,
    pub content: MessageContent,
    pub raw: Option<serde_json::Value>,  // 原始 payload（透传）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub text: String,
    pub mentions: Vec<String>,
    pub attachments: Vec<Attachment>,
}
```

### 消息回复目标

当插件要回复用户时，需要指定发送目标。`MessageTarget` 作为统一的回复地址：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTarget {
    pub platform: String,               // "qq"
    pub user_id: String,
    pub group_id: Option<String>,       // 群聊时非空
    pub channel_id: Option<String>,    // 频道消息时非空
    pub adapter_instance: Option<String>, // 目标适配器 name（回退到 platform）
}
```

当 host 收到插件的 send_message Action 时：
1. 使用 `MessageTarget.adapter_instance`（fallback `platform`）在 `AdapterRegistry` 中查找对应 Adapter
2. 调用 `adapter.send_message(target, content)` 发出

核心查找逻辑：

```rust
// adapter/registry.rs
pub struct AdapterRegistry {
    adapters: HashMap<String, Arc<dyn Adapter>>,
}

impl AdapterRegistry {
    pub fn register(&mut self, name: &str, adapter: Arc<dyn Adapter>) { ... }
    pub fn get(&self, name: &str) -> Option<Arc<dyn Adapter>> { ... }
    pub fn iter(&self) -> Iter<String, Arc<dyn Adapter>> { ... }
}
```

这种设计的考虑：
- **platform** 字段让插件可以感知来源，做差异逻辑
- **raw** 字段做逃生口，防止事件模型抽象遗漏细节
- **MessageContent** 标准化，但 `attachments` 保留扩展性

---

## 5. Wasm 插件系统

### 5.1 为何选择 Wasm

| 考量 | 结论 |
|------|------|
| 安全 | 沙箱执行，插件 crash 不影响主进程 |
| 生态 | Wasmtime 成熟活跃，官方 Rust SDK |
| 热加载 | 可替换 .wasm 文件后 reload，无需重启 |
| 语言无关 | 插件可以用 Rust / Go / C 等编写 |
| 性能 | JIT 编译，调用开销 ~微秒级 |

### 5.2 ABI 规范

插件需要导出的符号：

| 符号 | 签名 | 说明 |
|------|------|------|
| `plugin_version` | `() -> i32` | 返回 ABI 版本号 |
| `plugin_init` | `(ptr: i32, len: i32) -> i32` | 初始化，接收 JSON 配置 |
| `plugin_handle_event` | `(ptr: i32, len: i32) -> i64` | 处理事件，返回 packed (result_ptr << 32 \| result_len) |
| `plugin_destroy` | `() -> ()` | 卸载前清理 |

Host 函数（插件可以调用）：

| 符号 | 签名 | 说明 |
|------|------|------|
| `host_log` | `(level: i32, ptr: i32, len: i32)` | 日志输出 |
| `host_get_config` | `(ptr: i32, len: i32) -> i32` | 读取插件配置 |
| `host_send_message` | 暂未实现 | 后续迭代 |

序列化采用 JSON（简单通用，在 wasm 边界上性能可接受）。

### 5.3 PluginManager 设计

```rust
pub struct PluginManager {
    engine: Engine,
    plugins: HashMap<String, PluginInstance>,
    watchers: HashMap<PathBuf, NotifyWatcher>,
}

impl PluginManager {
    pub async fn load(&mut self, config: &PluginConfig) -> Result<()>;
    pub async fn unload(&mut self, name: &str) -> Result<()>;
    pub async fn reload(&mut self, name: &str) -> Result<()>;

    /// 并行分发事件给指定插件列表。各插件独立执行，互不影响。
    /// 返回所有插件产生的 Action 集合，core 逐个执行。
    pub async fn dispatch_parallel(
        &self,
        event: &Event,
        plugin_names: &[String],
    ) -> Vec<Action>;
}
```

### 5.4 并发模型与 Store 生命周期

Wasmtime 的 `Store` 不是 `Send + Sync`，而 `dispatch_parallel` 需要对同一插件下的多个事件（或不同插件间）安全并发调用。设计如下：

- **每个 `PluginInstance` 内部持有独立的 `Mutex<Store>`**，不同插件之间完全无锁竞争。
- 同一个插件的多次 `handle_event` 调用串行化在该插件自己的 Mutex 内（tokio task 会 suspend，不阻塞线程）。
- `Engine` 在所有插件间**共享**（`Engine` 是 `Send + Sync`），仅 `Store` 被隔离。

```rust
pub struct PluginInstance {
    name: String,
    store: Mutex<Store<PluginData>>,
    handle_event_fn: TypedFunc<(i32, i32), i64>,
    memory: Memory,
}
```

### 5.5 安全约束

- 每个插件有独立的 `Linker`，host 函数仅开放必要能力
- 配置中限制 CPU（fuel metering）和内存（memory limit）
- 所有 host 调用走 async，不阻塞 event loop

---

## 6. 配置系统

### 6.1 配置文件

使用 **TOML**。plugin 不再声明 routes，deploy 到 channel 时在 adapter 段指定绑定关系：

```toml
[core]
log_level = "info"

[[adapters]]
type = "qq"
enabled = true
name = "default"
[adapters.config]
app_id = "123456"
client_secret = "your_secret_here"

# 声明该 adapter 下有哪些 channel，每个 channel 绑定哪些插件
[[adapters.channels]]
channel_id = "group:123456"
plugins = ["builtin", "dice"]

[[adapters.channels]]
channel_id = "c2c:*"
plugins = ["builtin"]

[[plugins]]
name = "dice"
path = "./plugins/dice.wasm"
enabled = true
```

#### Channel ID 格式

`channel_id` 采用 `{type}:{id}` 格式，platform 由所属 adapter 确定：

| 示例 | 含义 |
|------|------|
| `group:123456` | QQ 群聊 123456 |
| `c2c:USER_OPENID` | QQ 私聊 |
| `group:*` | 通配所有群聊 |
| `*` | 全通配所有 channel |

channel_id 匹配规则：
- 精确匹配优先于通配匹配
- 同一个 channel 如果没有精确匹配，尝试 `"{type}:*"` 和 `"*"`
- 绑定表按 adapter name 分组，事件通过 `source_adapter` 定位到对应 adapter 的绑定
- 一个 channel 可以绑定多个插件，全部并行执行

### 6.2 配置热重载

- 使用 `notify` crate 监听配置文件变更
- 检测变更后执行 diff，动态增删 adapter / plugin

---

## 7. 启动流程

```
读取 CLI args ──► 加载配置文件 ──► 初始化 Core
                                      │
                ┌─────────────────────┼────────────────────┐
                ▼                     ▼                    ▼
        初始化 QQ Adapter    初始化其他 Adapter     加载 Plugin Wasm
        (WS 连接 + 鉴权)    (按配置)             (实例化 + init)
                │                     │                    │
                │                     │                    │
                └─────────────────────┼────────────────────┘
                                      ▼
                        构建 Channel Binding 表
                        (channel_id → [plugin_names])
                                      │
                                      ▼
                              Event Loop 启动
                              ┌──────────────────────────┐
                              │ event ← adapter TX       │
                              │ plugins ← binding.lookup │
                              │ actions ← parallel_dispatch │
                              │ execute_actions(actions) │
                              └──────────────────────────┘
```

Event Loop 核心：

```rust
// core/src/lib.rs
pub async fn run(self) -> Result<()> {
    let (event_tx, mut event_rx) = mpsc::channel(1024);

    for (name, adapter) in self.adapters.iter() {
        adapter.start(name.clone(), event_tx.clone(), self.shutdown.clone()).await?;
    }

    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                // 1. 根据 source_adapter 定位 adapter，查找 channel 绑定的插件
                let lookup = event.source_adapter.as_deref().unwrap_or(&event.platform);
                let plugin_names = self.bindings.resolve(lookup, &event.channel_id);

                // 2. 并行调用所有匹配的插件
                let actions = self.plugin_manager
                    .dispatch_parallel(&event, &plugin_names)
                    .await;

                // 3. 执行所有 Action（如发消息）
                for action in actions {
                    self.execute_action(action).await;
                }
            }
            _ = self.shutdown.cancelled() => break,
        }
    }
    Ok(())
}
```

---

## 8. 项目阶段规划

### Phase 1 — 骨架搭建

- [x] workspace + crate 结构初始化
- [x] `botadapt-core`: Config / Event / Adapter trait / Error
- [x] `botadapt-cli`: 最基本的 app 启动 + 日志
- [x] 配置文件解析（空 adapter，空 plugin）

### Phase 2 — QQ Adapter

- [x] 实现 `botadapt-qq` 的 WebSocket 连接
- [x] 事件推送接收 → 转换为统一 Event
- [x] API 封装（消息发送）
- [x] 集成进 core，端到端连通

### Phase 3 — Wasm 插件系统

- [x] Wasmtime 集成：Engine / Linker / Store
- [x] ABI 定义 + host 函数实现
- [x] `botadapt-plugin-sdk` 开发：类型 + host_calls + prelude
- [x] PluginManager + WASM 加载
- [x] 编写示例插件（dice）

### Phase 4 — 加固与扩展

- [ ] 配置热重载
- [ ] 错误恢复 / 重连机制
- [ ] 指标暴露（metrics）
- [ ] CI / 测试覆盖
- [ ] 文档 + 使用指南

---

## 9. 技术选型明细

| 领域 | 选型 | 理由 |
|------|------|------|
| 异步运行时 | Tokio | Rust 事实标准 |
| WebSocket | tokio-tungstenite | 纯异步，成熟 |
| HTTP 客户端 | reqwest | 异步，JSON 原生支持 |
| 序列化 | serde_json | 通用，Plugin ABI 也用它 |
| 配置解析 | toml / figment | 可读性强 |
| Wasm 引擎 | Wasmtime | 最成熟的 Rust Wasm runtime |
| 日志 | tracing | 异步感知，结构化 |
| 文件监听 | notify | 跨平台，debounce 支持 |
| 错误处理 | thiserror + color-eyre | 工程化错误展示 |

---

## 10. 设计原则

1. **Adapter 薄，Core 厚**：特定平台的逻辑尽量收在 adapter 内，公共能力（插件管理、生命周期、channel 绑定）由 core 提供。
2. **Plugin 无感知平台**：插件看到的是统一 Event，不需要知道来源是 QQ 还是 Discord。如确需差异，通过 `event.platform` 辨别。
3. **广播模型**：事件到达后，channel 绑定的所有插件并行执行，插件之间无依赖、无顺序。
4. **零配置可用**：默认配置 minimal，只启动 core 静默等待。
5. **优雅降级**：adapter 断线自动重连；单个插件崩溃不连累主进程，不影响同 channel 其他插件。
6. **异步贯穿始终**：从 WebSocket 接收到 Plugin dispatch 到 HTTP 回写，全链路 tokio async。
