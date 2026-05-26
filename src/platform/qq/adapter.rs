use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use botadapt_core::adapter::Adapter;
use botadapt_core::event::{AdapterEvent, MessageEvent, MessageMeta};

use crate::api::QqApi;
use crate::config::QQConfig;

pub struct QQAdapter {
    api: Arc<QqApi>,
}
impl QQAdapter {
    pub fn new(cfg: &toml::Table) -> anyhow::Result<Self> {
        let config = QQConfig::from_toml_table(cfg).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        return Ok(QQAdapter {
            api: QqApi::new_arc(&config),
        });
    }
}
#[async_trait]
impl Adapter for QQAdapter {
    async fn start(
        &self,
        emit: Box<dyn Fn(AdapterEvent) + Send + Sync + 'static>,
        shutdown: CancellationToken,
    ) -> anyhow::Result<()> {
        tracing::info!("QQ 适配器启动");

        let api = self.api.clone();
        let ws_shutdown = shutdown.clone();
        tokio::spawn(async move {
            crate::ws::client::run_loop(api, emit, ws_shutdown).await;
        });

        shutdown.cancelled().await;
        tracing::info!("QQ 适配器关闭");
        Ok(())
    }

    async fn send_message(&self, msg: &MessageEvent) -> anyhow::Result<()> {
        tracing::debug!(
            user_id = %match &msg.meta { MessageMeta::Private(p) => &p.user_id },
            text = %msg.content.text.chars().take(30).collect::<String>(),
            "QQAdapter::send_message"
        );
        match msg.meta.clone() {
            MessageMeta::Private(p) => {
                self.api
                    .send_c2c_message(&p.user_id, &msg.content.text, None)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
            }
        };
        Ok(())
    }
}
