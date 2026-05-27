pub mod binding;
pub mod bot;
pub mod builtin;
pub mod config;
pub mod events;
pub mod plugin;
pub mod session;

use bot::BotRegistry;
use plugin::PluginManager;
use session::SessionMgr;

use crate::{
    core::{
        binding::Bindings,
        events::{BotEvent, Message},
    },
    platform,
};
pub struct BotApp {
    cfg: config::Config,
    bots: BotRegistry,
    bindings: Bindings,
    session_mgr: SessionMgr,
    plugin_mgr: PluginManager,

    // plugin_manager: PluginManager,
    shutdown: tokio_util::sync::CancellationToken,
}

impl BotApp {
    /// 从配置文件构建
    pub fn from_config(cfg: config::Config) -> Self {
        let shutdown = tokio_util::sync::CancellationToken::new();
        let mut plugin_mgr = PluginManager::new();
        builtin::register_builtins(&mut plugin_mgr);
        let mut app = Self {
            cfg: cfg.clone(),
            bots: BotRegistry::new(shutdown.clone()),
            bindings: Bindings::new(cfg.bindings.clone()),
            session_mgr: SessionMgr::new(cfg.clone()),
            plugin_mgr: plugin_mgr,

            shutdown: shutdown,
            // plugin_manager: PluginManager::new(),
            // bindings: ChannelBinding::new(),
        };
        for bot_cfg in &app.cfg.bots {
            if !bot_cfg.enabled {
                continue;
            }
            match platform::new_bot_from_config(bot_cfg) {
                Ok(adapter) => {
                    app.bots.register(&bot_cfg.id, adapter);
                    tracing::info!(name = bot_cfg.id, "Bot创建成功");
                }
                Err(e) => tracing::error!("创建Bot失败：{}", e),
            }
        }
        app
    }

    /// 启动事件循环
    pub async fn run(&mut self) -> anyhow::Result<()> {
        let _ = self.bots.run();
        loop {
            tokio::select! {
                Some((bid, evt)) = self.bots.recv_bot_evt() => {
                    tracing::info!(bid, ?evt, "receive");

                    let BotEvent::Message(msg) = &evt;
                    let available: std::collections::HashSet<String> = self.plugin_mgr
                        .names()
                        .into_iter()
                        .map(String::from)
                        .collect();
                    tracing::info!(?available, ".");
                    let plugin_names = self.bindings.get_plugin_list(
                        &bid,
                        &msg.target_type,
                        &msg.target,
                        available,
                    );
                    tracing::info!(?plugin_names, ".");

                    for name in plugin_names {
                        if self.plugin_mgr.is_active(&name, &evt) {
                            if let Some(mut plugin) = self.plugin_mgr.get(&name) {
                                match plugin.handle(&evt) {
                                    Ok(action) => {
                                        if !action.reply.text.is_empty() {
                                            if let Some(bot) = self.bots.get(&bid) {
                                                if let Err(e) = bot.send_message(
                                                    &Message {
                                                        target: msg.target.clone(),
                                                        target_type: msg.target_type.clone(),
                                                        content: action.reply,
                                                    },
                                                ).await {
                                                    tracing::error!("发送消息失败: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("插件 {} 处理失败: {}", name, e);
                                    }
                                }
                            }
                        }
                    }
                }
                _ = self.shutdown.cancelled() => break,
            }
        }

        Ok(())
    }

    // pub fn shutdown(&self) {
    //     self.shutdown.cancel();
    // }

    // /// 执行单个 Action（如发消息）
    // async fn execute_action(&self, action: PluginEventWithName) {
    //     match action.event {
    //         PluginEvent::Message(e) => {
    //             let user_id = match &e.meta {
    //                 MessageMeta::Private(p) => p.user_id.clone(),
    //             };
    //             let text_snippet = e.content.text.chars().take(20).collect::<String>();
    //             let span = tracing::info_span!(
    //                 "send_message",
    //                 instance = %action.adapter_name,
    //                 user_id = %user_id,
    //                 text = %text_snippet,
    //             );
    //             async {
    //                 let adapter = self.adapters.get(&action.adapter_name);
    //                 if let Some(adapter) = adapter {
    //                     if let Err(e) = adapter.send_message(&e).await {
    //                         tracing::error!("发送消息失败: {}", e);
    //                     } else {
    //                         tracing::trace!("发送消息成功");
    //                     }
    //                 } else {
    //                     tracing::warn!("未找到适配器实例 {}", action.adapter_name);
    //                 }
    //             }
    //             .instrument(span)
    //             .await;
    //         }
    //     }
    // }
}
