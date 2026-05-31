# AGENTS.md

## Build & Run

```sh
cargo build
cargo test
cargo run                       # uses tinybot.toml by default
cargo run -- -c path/to/config.toml
```

To build a wasm plugin (example):
```sh
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2 -p hello
```

## Tech Stack

- Rust **edition 2024** (stable since 1.85)
- Async runtime: tokio (multi-thread, `#[tokio::main]`)
- Config: TOML with custom `${ENV_VAR:-default}` env var expansion (see `src/core/config/parser.rs`)
- Serialization: `serde` with `serde_inline_default` for field-level defaults (use `#[serde_inline_default(value)]`, not `#[serde(default = "...")]`)
- WASM runtime: `wasmtime` 45 with `component-model` feature (WASI P2)

## Architecture

The package is named `tinybot` (directory is `botadapt`).

```
src/
  main.rs           # entry: parse args, load config, init tracing, run BotApp
  args.rs           # single arg: -c/--config (default: tinybot.toml)
  core/
    config.rs       # Config, BotConfig, PluginConfig, BindingConfig structs
    config/parser.rs # ${VAR:-default} env expansion
    bot.rs          # Bot trait + BotRegistry (spawns each bot, collects events into mpsc)
    plugin.rs       # PluginFactory/Plugin traits (pure trait, no wasm awareness)
    plugin.wit       # WIT interface: defines the host↔guest contract for wasm plugins
    wasm.rs         # WasmPluginFactory/WasmPlugin (wraps wasmtime, implements traits)
    session.rs      # SessionMgr: (bot_id, target_type, target_id) -> Plugin instance
    binding.rs      # Bindings: glob-match routing of plugins to bots/targets
    events.rs       # BotEvent::Message { target, target_type, content }
  platform/
    stdio.rs        # debug platform: reads stdin, prints to stdout
    qq/             # QQ platform (incomplete)
plugins/
  hello/            # example wasm plugin (WASI P2 component)
    wit/world.wit   # same WIT as host-side plugin.wit
    src/lib.rs      # Guest implementation
```

## Plugin System (crucial to understand)

**All plugins are WASM components** (WASI P2). There are no built-in plugins.

**Two-phase lifecycle:**

1. **PluginFactory** (WasmPluginFactory): `active(&BotEvent) -> bool` decides if the plugin should handle a *new* message. Creates a temporary wasm instance to call `active`, then drops it. `create() -> Box<dyn Plugin>` spawns a persistent wasm instance.
2. **Plugin** (WasmPlugin): `handle(&BotEvent) -> Action { finish, reply }`. Holds a `Store` + instance that persists across multi-turn sessions.

A session is created when a plugin returns `Action { finish: false, ... }`. Subsequent messages in the same (bot, target_type, target_id) go directly to that Plugin instance. When `finish: true`, the session is destroyed (and the wasm instance is dropped).

**Binding routing:** `[[bindings]]` in config uses glob patterns (`*`, `group:*`, etc.) to match bot IDs, target types, and target IDs. The first matching binding's plugin list is used to find an active plugin.

### WIT Contract

The WIT file at `src/core/plugin.wit` defines the interface all plugins must implement:

```wit
interface plugin-api {
    use types.{action, bot-event};
    active: func(evt: bot-event) -> bool;
    handle: func(evt: bot-event) -> action;
}
```

The wasmtime `bindgen!` macro generates host-side bindings from this. The host is in `src/core/wasm.rs`. PluginManager only sees `PluginFactory`/`Plugin` traits — no wasmtime types leak.

### Engine Lifecycle

- `Engine`: global singleton (contains JIT compiler, must not be duplicated)
- `Component`: pre-compiled from `.wasm` file, shared across all sessions of a plugin
- `Store` + instance: created per session in `PluginFactory::create()`, persists across `handle()` calls

## Adding a New Platform

1. Create `src/platform/<name>.rs` implementing the `Bot` trait
2. Register it in `src/platform.rs` `new_bot_from_config()`

## Creating a New WASM Plugin

1. Create a new crate under `plugins/<name>/` with `crate-type = ["cdylib"]`
2. Depends on `wit-bindgen = "0.57"`
3. Copy `src/core/plugin.wit` to the guest's `wit/world.wit`
4. Implement `Guest` trait (from `exports::tinybot::plugin::plugin_api`)
5. Call `export!(MyStruct);`
6. Build with `--target wasm32-wasip2`
7. Add to `tinybot.toml`: `[[plugins]] name = "<name>" path = "./plugins/<name>/target/wasm32-wasip2/debug/<name>.wasm"`

## Testing

```sh
cargo test                          # all tests
cargo test -p tinybot -- config     # parser tests only
```

Tests use `temp_env` for environment variable isolation (see `src/core/config/parser.rs` tests).
