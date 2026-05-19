pub mod manager;
pub mod native;
pub mod wasm;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::event::{Event, MessageContent, MessageTarget};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    SendMessage {
        target: MessageTarget,
        content: MessageContent,
    },
    Noop,
}

#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;

    async fn handle_event(&self, event: Event) -> Result<Vec<Action>>;

    async fn init(&self, _config: &serde_json::Value) -> Result<()> {
        Ok(())
    }

    async fn destroy(&self) -> Result<()> {
        Ok(())
    }
}
