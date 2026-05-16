# AGENTS.md

## Architecture

BotAdapt is a Rust + Tokio async bot framework. Design doc: `DESIGN.md`.

**Distribution model**: events are **broadcast** to all plugins bound to a channel. Plugins execute **in parallel**, no ordering or dependency between them. No Router, no Break/Continue control flow.

**Channel Binding**: config maps `channel_id` → `[plugin_names]`, not event-type routes. `channel_id` format: `"{platform}:{type}:{id}"` (e.g. `"qq:group:123456"`). Supports wildcard `*`.

**Action**: pure side-effect (SendMessage, Noop). No control-flow semantics. Core collects all Actions from parallel dispatch and executes each.

## Workspace crates

| Crate | Role |
|-------|------|
| `botadapt-core` | Framework: Config, Event, EventBus, Adapter trait, PluginManager, ChannelBinding |
| `botadapt-qq` | QQ official bot API adapter (WebSocket + HTTP) |
| `botadapt-plugin-sdk` | Plugin dev library; `wasm32-wasip1` target; no_std |
| `botadapt-cli` | Binary entrypoint |

## Key types

- `Event` has `channel_id: String` (used to look up plugins) and `platform: String`
- `MessageTarget { platform, user_id, group_id?, channel_id? }` — host uses `platform` to find Adapter
- `Adapter` trait: `platform_id()`, `start(tx, shutdown)`, `send_message(target, content)`
- Wasm `Store` is per-`PluginInstance` wrapped in `Mutex` (not Send+Sync); `Engine` is shared

## Config

TOML format. Plugins bound per-channel inside adapter blocks:

```toml
[[adapters.channels]]
channel_id = "qq:group:123456"
plugins = ["echo", "admin"]
```

## Tech stack

Tokio, tokio-tungstenite, reqwest, serde_json, Wasmtime, tracing, notify, thiserror+color-eyre

## Design principles

1. Adapter thin, Core thick
2. Plugin platform-agnostic (use `event.platform` if needed)
3. Broadcast model: all bound plugins run in parallel
4. Graceful degradation: adapter auto-reconnect, plugin crash isolated
5. Async end-to-end
