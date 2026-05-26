use std::{collections::HashMap, net::Shutdown, sync::Arc};

use anyhow::Ok;
use async_trait::async_trait;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::core::events::BotEvent;

pub struct BotRegistry {
    adapters: HashMap<String, Arc<dyn Bot>>,
    shutdown: tokio_util::sync::CancellationToken,
    tx: UnboundedSender<(String, BotEvent)>,
    rx: UnboundedReceiver<(String, BotEvent)>,
}

#[async_trait]
pub trait Bot: Send + Sync {
    /// 启动适配器事件循环。
    ///
    /// 通过 `emit` 回调投递事件；回调内部负责设置 `source_adapter` 及发送。
    async fn start(
        &self,
        emit: Box<dyn Fn(super::events::BotEvent) + Send + Sync + 'static>,
        shutdown: tokio_util::sync::CancellationToken,
    ) -> anyhow::Result<()>;
    async fn send_message(&self, message: &super::events::Message) -> anyhow::Result<()>;
}

impl BotRegistry {
    pub fn new(cancel: tokio_util::sync::CancellationToken) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel::<(String, BotEvent)>();
        Self {
            adapters: HashMap::new(),
            shutdown: cancel,
            tx: event_tx,
            rx: event_rx,
        }
    }

    pub fn register(&mut self, bid: &str, bot: Arc<dyn Bot>) -> anyhow::Result<()> {
        if self.adapters.contains_key(bid) {
            anyhow::bail!("bot id 重复({})", bid);
        }
        self.adapters.insert(bid.to_owned(), bot.clone());

        Ok(())
    }
    pub fn run(&self) -> anyhow::Result<()> {
        for (bid, bot) in self.adapters.iter().map(|(a, b)| (a.clone(), b.clone())) {
            let tx = self.tx.clone();
            let bid = bid.to_owned();
            let shutdown = self.shutdown.clone();
            tokio::spawn(async move {
                let bid2 = bid.clone();
                if let Err(e) = bot
                    .start(
                        Box::new(move |event: BotEvent| {
                            if let Err(e) = tx.send((bid.clone(), event)) {
                                tracing::error!("消息发送失败 {}:{}", bid.clone(), e);
                            }
                        }),
                        shutdown,
                    )
                    .await
                {
                    tracing::error!(bid = bid2, "{}", e);
                }
            });
        }
        Ok(())
    }
    pub async fn recv_bot_evt(&mut self) -> Option<(String, BotEvent)> {
        self.rx.recv().await
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, Arc<dyn Bot>> {
        self.adapters.iter()
    }
    pub fn get(&self, name: &str) -> Option<Arc<dyn Bot>> {
        self.adapters.get(name).cloned()
    }
}
