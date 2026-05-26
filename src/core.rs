pub mod bot;
pub mod config;
pub mod events;

use std::net::Shutdown;

use bot::BotRegistry;
use tokio::sync::mpsc;

use crate::{
    core::events::{
        BotEvent::{self},
        Message, MessageContent,
    },
    platform,
};
pub struct BotApp {
    cfg: config::Config,
    bots: BotRegistry,
    // plugin_manager: PluginManager,
    // bindings: ChannelBinding,
    shutdown: tokio_util::sync::CancellationToken,
}

impl BotApp {
    /// 从配置文件构建
    pub fn from_config(cfg: config::Config) -> Self {
        let shutdown = tokio_util::sync::CancellationToken::new();
        let mut app = Self {
            cfg: cfg,
            bots: BotRegistry::new(shutdown.clone()),
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

    // /// 注册 Adapter 并绑定其 channels
    // pub fn register_adapter(
    //     &mut self,
    //     name: &str,
    //     adapter: Arc<dyn Adapter>,
    //     channels: &[ChannelEntry],
    // ) {
    //     for ch in channels {
    //         self.bindings
    //             .add(&name, ch.channel_id.clone(), ch.plugins.clone());
    //     }
    //     self.adapters.register(name, adapter);
    // }

    // /// 注册 Plugin（测试注入 Mock 用）
    // pub fn register_plugin(&mut self, name: &str, plugin: Box<dyn plugin::Plugin>) {
    //     self.plugin_manager.register(name, plugin);
    // }

    // /// 加载 WASM 插件
    // pub async fn load_wasm_plugin(
    //     &mut self,
    //     name: &str,
    //     path: &std::path::Path,
    //     config: serde_json::Value,
    // ) -> anyhow::Result<()> {
    //     self.plugin_manager.load_wasm(name, path, config).await
    // }

    // /// 并行批量加载 WASM 插件
    // pub async fn load_wasm_plugins(&mut self, plugins: &[config::PluginConfig]) {
    //     let engine = self.plugin_manager.engine().clone();
    //     let tasks: Vec<_> = plugins
    //         .iter()
    //         .filter(|p| p.enabled)
    //         .map(|cfg| {
    //             let name = cfg.name.clone();
    //             let path = std::path::PathBuf::from(&cfg.path);
    //             let engine = engine.clone();
    //             async move {
    //                 let wasm_bytes = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
    //                 let instance = tokio::task::spawn_blocking(move || {
    //                     PluginInstance::load(engine, &wasm_bytes, serde_json::json!({}))
    //                 })
    //                 .await
    //                 .map_err(|e| e.to_string())?
    //                 .map_err(|e| e.to_string())?;
    //                 Ok::<_, String>((name, instance))
    //             }
    //         })
    //         .collect();

    //     let results = futures::future::join_all(tasks).await;
    //     for result in results {
    //         match result {
    //             Ok((name, instance)) => {
    //                 self.plugin_manager
    //                     .register_wasm_instance(&name, Arc::new(instance));
    //                 tracing::info!("WASM 插件 {} 已加载", name);
    //             }
    //             Err(e) => {
    //                 tracing::error!("加载 WASM 插件失败: {}", e);
    //             }
    //         }
    //     }
    // }

    // /// 注册 Channel 绑定（测试注入用）
    // pub fn bind_channel(&mut self, name: &str, channel_id: &str, plugins: Vec<String>) {
    //     self.bindings.add(name, channel_id.to_string(), plugins);
    // }

    /// 启动事件循环
    pub async fn run(&mut self) -> anyhow::Result<()> {
        self.bots.run();
        loop {
            tokio::select! {
                Some((bid,evt)) = self.bots.recv_bot_evt() => {
                    tracing::info!("receive");
                    let span = tracing::info_span!(
                        "event",
                        %bid,
                        ?evt,
                    );
                  if let Some(b) =  self.bots.get(bid.as_str()){
                      b.send_message(
                          &Message{
                              target:"".to_owned(),
                              target_type:"".to_owned(),
                              content:MessageContent{
                                  text:"pong".to_owned(),
                              },
                          },
                      );
                  }

                //     async {
                //         tracing::debug!("收到事件");
                //         let adapt = event.adapter_name;
                //         let AdapterEvent::Message(m) = event.event.clone();
                //         let plugin_names = match m.meta {
                //             MessageMeta::Private(p) => {
                //                 self.bindings.resolve(&adapt, &p.user_id)
                //             }
                //         };
                //         tracing::debug!(
                //             user_id = %user_id,
                //             adapter = %adapt,
                //             plugins = plugin_names.len(),
                //             "channel 绑定解析"
                //         );

                //         if plugin_names.is_empty() {
                //             tracing::debug!("channel 无绑定插件");
                //             return;
                //         }

                //         let actions = self
                //             .plugin_manager
                //             .dispatch_parallel(&event.event, &plugin_names)
                //             .await;

                //         for action in actions {
                //             self.execute_action(PluginEventWithName {
                //                 adapter_name: adapt.clone(),
                //                 event: action,
                //             })
                //             .await;
                //         }
                //     }
                //     .instrument(span)
                //     .await;
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
