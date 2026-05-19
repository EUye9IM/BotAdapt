use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use botadapt_core::adapter::Adapter;
use botadapt_core::error::Result;
use botadapt_core::event::{Event, MessageContent, MessageTarget};

use crate::api::QqApi;
use crate::config::QQConfig;

use crate::PLATFORM_ID;

pub struct QQAdapter {
    name: String,
    api: Arc<QqApi>,
}

impl QQAdapter {
    pub fn new(config: QQConfig, name: String) -> Self {
        Self {
            name,
            api: QqApi::new_arc(&config),
        }
    }

    pub fn new_arc(config: QQConfig, name: String) -> Arc<dyn Adapter> {
        Arc::new(Self::new(config, name))
    }
}

#[async_trait]
impl Adapter for QQAdapter {
    fn platform_id(&self) -> &'static str {
        PLATFORM_ID
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    async fn start(&self, tx: mpsc::Sender<Event>, shutdown: CancellationToken) -> Result<()> {
        tracing::info!(
            name = %self.name,
            "QQ 适配器启动"
        );

        let api = self.api.clone();
        let ws_shutdown = shutdown.clone();
        let name = self.name.clone();
        tokio::spawn(async move {
            crate::ws::client::run_loop(api, tx, ws_shutdown, name).await;
        });

        shutdown.cancelled().await;
        tracing::info!(
            name = %self.name,
            "QQ 适配器关闭"
        );
        Ok(())
    }

    async fn send_message(
        &self,
        target: &MessageTarget,
        content: &MessageContent,
    ) -> Result<()> {
        tracing::debug!(
            name = %self.name,
            user_id = %target.user_id,
            group_id = ?target.group_id,
            text = %content.text.chars().take(30).collect::<String>(),
            "QQAdapter::send_message"
        );
        self.api
            .send_c2c_message(&target.user_id, &content.text, None)
            .await
            .map_err(|e| botadapt_core::error::Error::Adapter(e.to_string()))?;
        Ok(())
    }
}
