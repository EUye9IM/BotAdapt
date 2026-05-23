//! SDK 类型定义，与 botadapt-core 保持结构一致以实现 JSON 序列化互通。

mod event;

pub use event::{AdapterEvent, MessageContent, MessageEvent, MessageMeta, PluginEvent, PrivateMeta};
