pub mod adapter;
pub mod binding;
pub mod config;
pub mod error;
pub mod event;
pub mod plugin;

use std::sync::Arc;

use adapter::registry::AdapterRegistry;
use adapter::Adapter;
use binding::ChannelBinding;
use error::Result;
use event::Event;
use plugin::manager::PluginManager;
use plugin::Action;
use tokio::sync::mpsc;
use tracing::Instrument;

pub struct BotApp {
    #[allow(dead_code)]
    config: config::Config,
    adapters: AdapterRegistry,
    plugin_manager: PluginManager,
    bindings: ChannelBinding,
    shutdown: tokio_util::sync::CancellationToken,
}

impl BotApp {
    /// 从配置文件构建（生产用）
    pub fn from_config(cfg: config::Config) -> Self {
        let mut bindings = ChannelBinding::new();
        let plugin_manager = PluginManager::new();
        let adapters = AdapterRegistry::new();

        // 构建 channel → plugin 绑定表
        for adapter_cfg in &cfg.adapters {
            for ch in &adapter_cfg.channels {
                bindings.add(ch.channel_id.clone(), ch.plugins.clone());
            }
        }

        Self {
            config: cfg,
            adapters,
            plugin_manager,
            bindings,
            shutdown: tokio_util::sync::CancellationToken::new(),
        }
    }

    /// 空白构建（测试用，外部注入 Mock 组件）
    pub fn empty() -> Self {
        Self {
            config: config::Config {
                core: config::CoreConfig::default(),
                adapters: Vec::new(),
                plugins: Vec::new(),
            },
            adapters: AdapterRegistry::new(),
            plugin_manager: PluginManager::new(),
            bindings: ChannelBinding::new(),
            shutdown: tokio_util::sync::CancellationToken::new(),
        }
    }

    /// 注册 Adapter（测试注入 Mock 用）
    pub fn register_adapter(&mut self, adapter: Arc<dyn Adapter>) {
        self.adapters.register(adapter);
    }

    /// 注册 Plugin（测试注入 Mock 用）
    pub fn register_plugin(&mut self, plugin: Box<dyn plugin::Plugin>) {
        self.plugin_manager.register(plugin);
    }

    /// 注册 Channel 绑定（测试注入用）
    pub fn bind_channel(&mut self, channel_id: &str, plugins: Vec<String>) {
        self.bindings.add(channel_id.to_string(), plugins);
    }

    /// 启动事件循环
    pub async fn run(self) -> Result<()> {
        if self.adapters.is_empty() && self.bindings.is_empty() {
            tracing::info!("无适配器注册，静默等待");
        }

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(1024);

        // 启动所有 Adapter
        let platforms: Vec<String> = self.adapters.platforms().map(|s| s.to_string()).collect();
        for platform in platforms {
            if let Some(adapter) = self.adapters.get(&platform) {
                let tx = event_tx.clone();
                let shutdown = self.shutdown.clone();
                let adapter = adapter.clone();
                tokio::spawn(async move {
                    if let Err(e) = adapter.start(tx, shutdown).await {
                        tracing::error!("适配器 {} 启动失败: {}", platform, e);
                    }
                });
            }
        }

        loop {
            tokio::select! {
                Some(event) = event_rx.recv() => {
                    let span = tracing::info_span!(
                        "event",
                        event_id = %event.id,
                        channel_id = %event.channel_id,
                        platform = %event.platform,
                    );

                    async {
                        tracing::debug!("收到事件");

                        let plugin_names = self.bindings.resolve(&event.channel_id);
                        tracing::debug!(
                            "channel 绑定解析: {} → {} 个插件",
                            event.channel_id,
                            plugin_names.len()
                        );

                        if plugin_names.is_empty() {
                            tracing::debug!("channel {} 无绑定插件", event.channel_id);
                            return;
                        }

                        let actions = self.plugin_manager.dispatch_parallel(&event, &plugin_names).await;

                        for action in actions {
                            self.execute_action(action).await;
                        }
                    }
                    .instrument(span)
                    .await;
                }
                _ = self.shutdown.cancelled() => break,
            }
        }

        Ok(())
    }

    pub fn shutdown(&self) {
        self.shutdown.cancel();
    }

    /// 执行单个 Action（如发消息）
    async fn execute_action(&self, action: Action) {
        match action {
            Action::SendMessage { target, content } => {
                let span = tracing::info_span!(
                    "send_message",
                    platform = %target.platform,
                    user_id = %target.user_id,
                    text = %content.text.chars().take(20).collect::<String>(),
                );
                async {
                    if let Some(adapter) = self.adapters.get(&target.platform) {
                        if let Err(e) = adapter.send_message(&target, &content).await {
                            tracing::error!("发送消息失败: {}", e);
                        } else {
                            tracing::trace!("发送消息成功");
                        }
                    } else {
                        tracing::warn!("未找到平台 {} 的适配器", target.platform);
                    }
                }
                .instrument(span)
                .await;
            }
            Action::Noop => {}
        }
    }
}
