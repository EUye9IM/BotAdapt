use crate::core::events::{BotEvent, Message, MessageContent};
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::sync::CancellationToken;

pub const PLAT: &str = "debug";

// 测试接口，从标准输入输出读写
pub struct Bot {}
impl Bot {
    pub fn new(cfg: &toml::Table) -> anyhow::Result<Self> {
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
        shutdown.cancelled().await;
        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();
        loop {
            tokio::select! {
                line = lines.next_line() => {
                    match line {
                        Ok(Some(content)) => {
                            emit(
                                BotEvent::Message(
                                    Message {
                                        target_type: "".to_owned(),
                                        target: "".to_owned(),
                                        content:MessageContent{
                                            text: content,
                                        } ,
                                    },
                                ),
                            );
                        },
                        Ok(None) => {},
                        Err(e) => tracing::error!("IO 错误: {}", e),
                    }
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
        print!("{:?}", &msg.content);
        Ok(())
    }
}
