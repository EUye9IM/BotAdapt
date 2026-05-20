use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use botadapt_core::adapter::Adapter;
use botadapt_core::error::{Error, Result};
use botadapt_core::event::{Event, MessageContent, MessageTarget};

use crate::api::QqApi;
use crate::config::QQConfig;

use crate::error::QqError;
use crate::PLATFORM_ID;

pub struct QQAdapter {
    api: Arc<QqApi>,
}
impl QQAdapter {
    pub fn new(cfg: &toml::Table) -> Result<Self> {
        let config = QQConfig::from_toml_table(cfg).map_err(|e| Error::Config(e.to_string()))?;
        return Ok(QQAdapter {
            api: QqApi::new_arc(&config),
        });
    }
}
#[async_trait]
impl Adapter for QQAdapter {
    async fn start(
        &self,
        self_name: String,
        tx: mpsc::Sender<Event>,
        shutdown: CancellationToken,
    ) -> Result<()> {
        tracing::info!("QQ 适配器启动");

        let api = self.api.clone();
        let ws_shutdown = shutdown.clone();
        tokio::spawn(async move {
            crate::ws::client::run_loop(api, tx, ws_shutdown, self_name).await;
        });

        shutdown.cancelled().await;
        tracing::info!("QQ 适配器关闭");
        Ok(())
    }

    async fn send_message(&self, target: &MessageTarget, content: &MessageContent) -> Result<()> {
        tracing::debug!(
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
