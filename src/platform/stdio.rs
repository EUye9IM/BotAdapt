use crate::core::events::{BotEvent, Message, MessageContent};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub const PLAT: &str = "stdio";

// 测试接口，从标准输入输出读写
pub struct Bot {}
impl Bot {
    pub fn new(_cfg: &toml::Table) -> anyhow::Result<Self> {
        return Ok(Bot {});
    }
}
#[async_trait]
impl crate::core::bot::Bot for Bot {
    async fn start(
        &self,
        emit: Box<dyn Fn(BotEvent) + Send + Sync + 'static>,
        shutdown: CancellationToken,
    ) -> anyhow::Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        tokio::task::spawn_blocking(move || {
            let stdin = std::io::stdin();
            let mut line = String::new();
            while stdin.read_line(&mut line).is_ok() {
                let trimmed = line.trim_end().to_string();
                line.clear();
                if tx.send(trimmed).is_err() {
                    break;
                }
            }
        });

        loop {
            tokio::select! {
                Some(content) = rx.recv() => {
                    emit(BotEvent::Message(Message {
                        target_type: "debug".to_owned(),
                        target: "".to_owned(),
                        content: MessageContent { text: content },
                    }));
                }
                _ = shutdown.cancelled() => {
                    tracing::info!("收到取消");
                    break;
                }
            }
        }
        Ok(())
    }

    async fn send_message(&self, msg: &Message) -> anyhow::Result<()> {
        println!("{}", &msg.content.text);
        Ok(())
    }
}
