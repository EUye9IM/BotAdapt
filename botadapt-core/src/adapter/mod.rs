pub mod registry;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::event::{Event, MessageContent, MessageTarget};

#[async_trait]
pub trait Adapter: Send + Sync {
    /// 启动适配器事件循环。
    ///
    /// 通过 `emit` 回调投递事件；回调内部负责设置 `source_adapter` 及发送。
    async fn start(
        &self,
        emit: Box<dyn Fn(Event) + Send + Sync + 'static>,
        shutdown: CancellationToken,
    ) -> Result<()>;
    async fn send_message(&self, target: &MessageTarget, content: &MessageContent) -> Result<()>;
}
