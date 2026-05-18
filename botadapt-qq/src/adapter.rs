use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use botadapt_core::adapter::Adapter;
use botadapt_core::error::Result;
use botadapt_core::event::{Event, MessageContent, MessageTarget};

use crate::api::QqApi;
use crate::config::QQConfig;

pub struct QQAdapter {
    instance_id: String,
    api: Arc<QqApi>,
}

impl QQAdapter {
    pub fn new(config: QQConfig) -> Self {
        let instance_id = format!("qq:{}", config.name.as_deref().unwrap_or(&config.app_id));
        Self {
            instance_id,
            api: QqApi::new_arc(&config),
        }
    }

    pub fn new_arc(config: QQConfig) -> Arc<dyn Adapter> {
        Arc::new(Self::new(config))
    }
}

#[async_trait]
impl Adapter for QQAdapter {
    fn platform_id(&self) -> &'static str {
        "qq"
    }

    fn instance_id(&self) -> String {
        self.instance_id.clone()
    }

    async fn start(&self, tx: mpsc::Sender<Event>, shutdown: CancellationToken) -> Result<()> {
        tracing::info!(
            instance_id = %self.instance_id,
            "QQ 适配器启动"
        );

        let api = self.api.clone();
        let ws_shutdown = shutdown.clone();
        let instance_id = self.instance_id.clone();
        tokio::spawn(async move {
            crate::ws::client::run_loop(api, tx, ws_shutdown, instance_id).await;
        });

        shutdown.cancelled().await;
        tracing::info!(
            instance_id = %self.instance_id,
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
            instance_id = %self.instance_id,
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
