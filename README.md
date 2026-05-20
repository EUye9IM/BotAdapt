# BotAdapt

通用异步 Bot 框架，基于 Rust + Tokio。平台无关的事件驱动 + Wasm 插件化。

## 设计

详见 [DESIGN.md](DESIGN.md) 和 [AGENTS.md](AGENTS.md)。

## 结构

| Crate | 说明 |
|-------|------|
| `botadapt-core` | 框架核心：Event、Adapter trait、PluginManager、ChannelBinding |
| `botadapt-qq` | QQ 官方 Bot API 适配器 |
| `botadapt-cli` | CLI 入口 |
| `botadapt-plugin-sdk` | 插件开发 SDK（wasm32-wasip1） |

## 快速开始

```bash
# 编译
cargo build

# 运行（需要配置文件）
cargo run -- botadapt.toml

# 构建 dice 插件（需要 wasm 目标）
rustup target add wasm32-wasip1
cargo build -p dice --target wasm32-wasip1 --release
cp target/wasm32-wasip1/release/dice.wasm plugins/
```

## 配置

TOML 格式，channel 绑定在所属 adapter 下：

```toml
[[adapters]]
type = "qq"
enabled = true
name = "default"
[adapters.config]
app_id = "${QQ_APP_ID}"
client_secret = "${QQ_CLIENT_SECRET}"

[[adapters.channels]]
channel_id = "group:*"
plugins = ["builtin", "dice"]
```

## 进度

- [x] Phase 1 — 骨架搭建
- [x] Phase 2 — QQ Adapter
- [x] Phase 3 — Wasm 插件系统（运行时 + dice 示例）
- [ ] Phase 4 — 加固与扩展
