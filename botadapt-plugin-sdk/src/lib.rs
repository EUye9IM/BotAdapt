//! botadapt-plugin-sdk
//!
//! 插件开发 SDK，编译目标为 wasm32-wasip1。
//! 提供 AdapterEvent / PluginEvent / MessageEvent 等类型及 host 函数声明。

pub mod host_calls;
pub mod prelude;

mod types;

pub use types::*;
