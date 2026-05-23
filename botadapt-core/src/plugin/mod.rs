pub mod manager;
pub mod native;
pub mod wasm;
use crate::event::{AdapterEvent, PluginEvent};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Plugin: Send + Sync {
    async fn handle_event(&self, event: AdapterEvent) -> Result<Vec<PluginEvent>>;

    async fn init(&self, _config: &serde_json::Value) -> anyhow::Result<()> {
        Ok(())
    }

    async fn destroy(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
