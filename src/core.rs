pub mod binding;
pub mod bot;
pub mod config;
pub mod events;
mod hello;
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

    fn handle_builtin(&self, evt: &BotEvent) -> Option<Action> {
        let BotEvent::Message(msg) = evt;
        match msg.content.text.trim() {
            "/ping" => Some(Action {
                finish: true,
                reply: Some(MessageContent {
                    text: "pong!".to_owned(),
                }),
            }),
            _ => None,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let _ = self.bots.run();
        hello::register_builtins(&mut self.plugin_mgr);
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
                    let mut new_plugin = None;
                    let mut has_existing_session = false;

                    let action = match self.handle_builtin(&evt) {
                        Some(a) => Ok(a),
                        None => match self.session_mgr.get_session(&bid, &msg.target_type, &msg.target) {
                            Some(sess) => {
                                tracing::debug!("get session");
                                has_existing_session = true;
                                sess.plugin.handle(&evt)
                            }
                            None => {
                                let mut act = Ok(Action {
                                    finish: true,
                                    reply: None,
                                });
                                let plugin_list = self.bindings.get_plugin_list(
                                    &bid,
                                    &msg.target_type,
                                    &msg.target,
                                    available_plugin.clone(),
                                );
                                for name in plugin_list {
                                    if self.plugin_mgr.is_active(&name, &evt) {
                                        if let Some(mut plugin) = self.plugin_mgr.get(&name) {
                                            act = plugin.handle(&evt);
                                            new_plugin = Some(plugin);
                                            break;
                                        }
                                    }
                                }
                                act
                            }
                        }
                    };
                    tracing::debug!("{:?}",action);
                    match action {
                        Ok(a) => {
                            if let Some(content) = a.reply {
                                if let Some(bot) = self.bots.get(&bid) {
                                    let bot = bot.clone();
                                    let msg = Message {
                                        target: msg.target.clone(),
                                        target_type: msg.target_type.clone(),
                                        content,
                                    };
                                    tokio::spawn(async move {
                                        if let Err(e) = bot.send_message(&msg).await {
                                            tracing::error!("发送消息失败: {}", e);
                                        }
                                    });
                                }
                            }
                            if has_existing_session && a.finish {
                                tracing::debug!("del session");
                                self.session_mgr.remove_session(&bid, &msg.target_type, &msg.target);
                            } else if let Some(plugin) = new_plugin {
                                if !a.finish {
                                    tracing::debug!("new session");
                                    self.session_mgr.create_session(
                                        &bid,
                                        &msg.target_type,
                                        &msg.target,
                                        plugin,
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("消息处理失败: {}", e);
                            if has_existing_session {
                                self.session_mgr.remove_session(&bid, &msg.target_type, &msg.target);
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
