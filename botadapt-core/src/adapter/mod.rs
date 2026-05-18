pub mod registry;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::event::{Event, MessageContent, MessageTarget};

#[async_trait]
pub trait Adapter: Send + Sync {
    fn platform_id(&self) -> &'static str;

    fn instance_id(&self) -> String {
        self.platform_id().to_string()
    }

    async fn start(&self, tx: mpsc::Sender<Event>, shutdown: CancellationToken) -> Result<()>;

    async fn send_message(
        &self,
        target: &MessageTarget,
        content: &MessageContent,
    ) -> Result<()>;
}
