use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use botadapt_core::adapter::Adapter;
use botadapt_core::error::Result;
use botadapt_core::event::{Event, MessageContent, MessageTarget};

pub struct QQAdapter;

impl QQAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn new_arc() -> Arc<dyn Adapter> {
        Arc::new(Self)
    }
}

#[async_trait]
impl Adapter for QQAdapter {
    fn platform_id(&self) -> &'static str {
        "qq"
    }

    async fn start(&self, _tx: mpsc::Sender<Event>, shutdown: CancellationToken) -> Result<()> {
        tracing::info!("QQ 适配器启动");
        // Phase 2: 实现 WebSocket 连接 + 事件转换
        shutdown.cancelled().await;
        tracing::info!("QQ 适配器关闭");
        Ok(())
    }

    async fn send_message(
        &self,
        target: &MessageTarget,
        content: &MessageContent,
    ) -> Result<()> {
        tracing::info!("QQ 发送消息 -> {}: {}", target.user_id, content.text);
        // Phase 2: 实现 QQ HTTP API 调用
        Ok(())
    }
}
