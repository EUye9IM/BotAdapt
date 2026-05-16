//! SDK 类型定义，与 botadapt-core 保持结构一致以实现 JSON 序列化互通。

mod action;
mod event;
mod target;

pub use action::Action;
pub use event::{Event, EventKind, MessageContent, MessageEvent};
pub use target::MessageTarget;
