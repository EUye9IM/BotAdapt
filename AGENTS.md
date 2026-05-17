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

TOML format. Supports `${ENV_VAR}` environment variable expansion with
optional defaults `${ENV_VAR:-default}`. Expansion happens after TOML parse
so values with special characters (quotes, newlines) are safe.

```toml
[[adapters]]
type = "qq"
[adapters.config]
app_id = "${QQ_APP_ID}"
client_secret = "${QQ_CLIENT_SECRET}"

[[adapters.channels]]
channel_id = "qq:group:123456"
plugins = ["echo", "admin"]
```

## Tech stack

Tokio, tokio-tungstenite, reqwest, serde_json, Wasmtime, tracing, notify, thiserror+color-eyre

## Logging standard

Uses `tracing` crate. **Every `ERROR` must be logged before returned; every external call (HTTP, WS) must have DEBUG log of request/response.**

### Log levels

| Level | When | Examples |
|-------|------|----------|
| `ERROR` | Irrecoverable, impacts service | adapter start failure, token refresh failure, plugin crash |
| `WARN` | Recoverable, auto-retry/degrade | WS disconnect (will reconnect), API non-200, no adapter found |
| `INFO` | State change, key milestones | startup/shutdown, WS ready (session_id), plugin register |
| `DEBUG` | Request/response details, dispatch | Identify/Ready content, event conversion, API status codes, channel lookup result |
| `TRACE` | Per-message protocol details | every WS message opcode, heartbeat tick, seq change, raw JSON payload |

### Span rules

**Must use span** to correlate logs across concurrent events. Every `Event` dispatch creates a span. Every HTTP call uses `#[tracing::instrument]`. Every plugin execution creates a child span.

```
Event span (event_id, channel_id, platform)
  ├── Plugin span (plugin=name)
  │     ├── Adapter::send_message span (user_id, text_snippet)
  │     └── ...
  └── Plugin span (plugin=name2)
```

Subscriber must enable span enter/close events:

```rust
tracing_subscriber::fmt()
    .with_span_events(FmtSpan::FULL)
    .init();
```

Or at minimum `FmtSpan::NEW | FmtSpan::CLOSE` for lifecycle visibility.

### Per-module requirements

**api/mod.rs** (QqApi): Every HTTP call must log at DEBUG: method + URL + status. Token operations at DEBUG: cache hit/miss + remaining TTL. ERROR on failure.

**ws/client.rs**: INFO on connection ready (session_id). WARN on disconnect (with retry count). DEBUG for Identify/Ready content. TRACE for every incoming WS message (opcode, s, t).

**ws/heartbeat.rs**: TRACE on each tick (interval + seq sent).

**event/converter.rs**: DEBUG on conversion result (channel_id, content snippet). TRACE on raw input JSON.

**adapter.rs**: INFO on start/stop. DEBUG on send_message (target, content snippet).

**core event loop**: INFO span on each event. DEBUG for channel binding lookup result. TRACE for action execution.

### Subscriber init

Config uses `${RUST_LOG:-info}` expansion for runtime override via env var.
Requires `tracing-subscriber` with `env-filter` feature (not in default features for 0.3.x).

```toml
# config/default.toml
[core]
log_level = "${RUST_LOG:-info}"
```

```rust
// botadapt-cli/src/main.rs
use tracing_subscriber::EnvFilter;

let filter = EnvFilter::new(&config.core.log_level);
tracing_subscriber::fmt()
    .with_env_filter(filter)
    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
    .init();
```

## Design principles

1. Adapter thin, Core thick
2. Plugin platform-agnostic (use `event.platform` if needed)
3. Broadcast model: all bound plugins run in parallel
4. Graceful degradation: adapter auto-reconnect, plugin crash isolated
5. Async end-to-end
