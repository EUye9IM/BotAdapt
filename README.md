# tinybot

多平台 bot 框架，所有插件以 WASI P2 (WebAssembly Component Model) 形式加载。

## 快速开始

需要 Rust 1.85+ 和 `wasm32-wasip2` target：

```sh
rustup target add wasm32-wasip2
cargo build
cargo run                       # 默认读取 tinybot.toml
cargo run -- -c path/to/config.toml
```

## 配置

配置使用 TOML 格式，支持 `${ENV_VAR:-default}` 环境变量展开。

```toml
[core]
log_level = "${RUST_LOG:-tinybot=info}"

[[bots]]
id = "debug"
platform = "stdio"

[[plugins]]
name = "hello"
path = "./plugins/hello/target/wasm32-wasip2/debug/hello.wasm"

[[bindings]]
botid = "*"
target_type = "*"
target_id = "*"
plugins = ["*"]
```

### 配置段

| 段 | 说明 |
|---|---|
| `[core]` | `log_level`: 日志级别，支持 env filter 语法 |
| `[[bots]]` | `id`/`platform`/`config`: 注册平台适配器 |
| `[[plugins]]` | `name`/`path`/`enabled`: 注册 wasm 插件 |
| `[[bindings]]` | `botid`/`target_type`/`target_id`/`plugins`: glob 匹配路由插件 |

## 插件系统

所有插件都是 WASI P2 组件。接口定义在 `wit/plugin.wit`：

- `active(evt)` → `bool` — 判断插件是否处理该消息
- `handle(evt)` → `action` — 处理消息，返回 `{ finish, reply }`
- `finish: false` 时创建 session，后续同 (bot, target) 的消息直接路由到该 session

### 创建新插件

```sh
# 以 plugins/hello 为例，复制到新目录
cp -r plugins/hello plugins/myplugin

# 编辑 Cargo.toml 和 src/lib.rs，实现 Guest trait
# 构建
cargo build --target wasm32-wasip2 -p myplugin

# 在 tinybot.toml 中注册
# [[plugins]]
# name = "myplugin"
# path = "./plugins/myplugin/target/wasm32-wasip2/debug/myplugin.wasm"
```

## 添加平台适配器

1. 创建 `src/platform/<name>.rs`，实现 `Bot` trait
2. 在 `src/platform.rs` 的 `new_bot_from_config()` 中注册
3. 在 `tinybot.toml` 中配置 `[[bots]]`

## 架构

详见 [AGENTS.md](./AGENTS.md)。
