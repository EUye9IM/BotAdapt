pub mod registry;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::event::{AdapterEvent, MessageEvent};

#[async_trait]
pub trait Adapter: Send + Sync {
    /// 启动适配器事件循环。
    ///
    /// 通过 `emit` 回调投递事件；回调内部负责设置 `source_adapter` 及发送。
    async fn start(
        &self,
        emit: Box<dyn Fn(AdapterEvent) + Send + Sync + 'static>,
        shutdown: CancellationToken,
    ) -> anyhow::Result<()>;
    async fn send_message(&self, message: &MessageEvent) -> anyhow::Result<()>;
}
