pub mod registry;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::event::{Event, MessageContent, MessageTarget};

#[async_trait]
pub trait Adapter: Send + Sync {
    /// 启动适配器事件循环。
    ///
    /// `self_name` 为适配器在注册表中的名称（如 `"default"`），
    /// 用于事件溯源（写入 `Event.source_adapter`）及日志标识。
    async fn start(
        &self,
        self_name: String,
        tx: mpsc::Sender<Event>,
        shutdown: CancellationToken,
    ) -> Result<()>;
    async fn send_message(&self, target: &MessageTarget, content: &MessageContent) -> Result<()>;
}
