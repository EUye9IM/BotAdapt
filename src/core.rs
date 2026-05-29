pub mod binding;
pub mod bot;
pub mod config;
pub mod events;
pub mod plugin;
pub mod session;

use bot::BotRegistry;
use plugin::{Action, PluginManager};
use session::SessionMgr;

use crate::{
    core::{
        binding::Bindings,
        events::{BotEvent, Message, MessageContent},
    },
    platform,
};
pub struct BotApp {
    cfg: config::Config,
    bots: BotRegistry,
    bindings: Bindings,
    session_mgr: SessionMgr,
    plugin_mgr: PluginManager,
    shutdown: tokio_util::sync::CancellationToken,
}

impl BotApp {
    /// 从配置文件构建
    pub fn from_config(cfg: config::Config) -> Self {
        let shutdown = tokio_util::sync::CancellationToken::new();
        let plugin_mgr = PluginManager::new();
        let mut app = Self {
            cfg: cfg.clone(),
            bots: BotRegistry::new(shutdown.clone()),
            bindings: Bindings::new(cfg.bindings.clone()),
            session_mgr: SessionMgr::new(),
            plugin_mgr: plugin_mgr,

            shutdown: shutdown,
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

    /// 处理 builtin 命令，返回 Option<Action>
    fn handle_builtin(&self, evt: &BotEvent) -> Option<Action> {
        let BotEvent::Message(msg) = evt;
        match msg.content.text.trim() {
            "/ping" => Some(Action {
                finish: true,
                reply: MessageContent {
                    text: "pong!".to_owned(),
                },
            }),
            _ => None,
        }
    }

    /// 启动事件循环
    pub async fn run(&mut self) -> anyhow::Result<()> {
        let _ = self.bots.run();
        let available_plugin: std::collections::HashSet<String> = self
            .plugin_mgr
            .names()
            .into_iter()
            .map(String::from)
            .collect();
        loop {
            tokio::select! {
                Some((bid, evt)) = self.bots.recv_bot_evt() => {
                    tracing::info!(bid, ?evt, "receive");

                    let BotEvent::Message(msg) = &evt;

                    if let Some(action) = self.handle_builtin(&evt) {
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
                        continue;
                    }

                    if let Some(session) = self.session_mgr.get_session(&bid, &msg.target_type, &msg.target) {
                        match session.plugin.handle(&evt) {
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
                                if action.finish {
                                    self.session_mgr.remove_session(&bid, &msg.target_type, &msg.target);
                                }
                            }
                            Err(e) => {
                                tracing::error!("会话插件处理失败: {}", e);
                                self.session_mgr.remove_session(&bid, &msg.target_type, &msg.target);
                            }
                        }
                    } else {
                        let plugin_names = self.bindings.get_plugin_list(
                            &bid,
                            &msg.target_type,
                            &msg.target,
                            available_plugin.clone(),
                        );

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
                                            if !action.finish {
                                                self.session_mgr.create_session(
                                                    &bid,
                                                    &msg.target_type,
                                                    &msg.target,
                                                    plugin,
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("插件 {} 处理失败: {}", name, e);
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
                _ = self.shutdown.cancelled() => break,
            }
        }

        Ok(())
    }
}
