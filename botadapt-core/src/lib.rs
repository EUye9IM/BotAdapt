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
use config::ChannelEntry;
use error::Result;
use event::Event;
use plugin::manager::PluginManager;
use plugin::wasm::PluginInstance;
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
        Self {
            config: cfg,
            adapters: AdapterRegistry::new(),
            plugin_manager: PluginManager::new(),
            bindings: ChannelBinding::new(),
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

    /// 注册 Adapter 并绑定其 channels
    pub fn register_adapter(
        &mut self,
        name: &str,
        adapter: Arc<dyn Adapter>,
        channels: &[ChannelEntry],
    ) {
        for ch in channels {
            self.bindings
                .add(&name, ch.channel_id.clone(), ch.plugins.clone());
        }
        self.adapters.register(name, adapter);
    }

    /// 注册 Plugin（测试注入 Mock 用）
    pub fn register_plugin(&mut self, plugin: Box<dyn plugin::Plugin>) {
        self.plugin_manager.register(plugin);
    }

    /// 加载 WASM 插件
    pub async fn load_wasm_plugin(
        &mut self,
        name: &str,
        path: &std::path::Path,
        config: serde_json::Value,
    ) -> Result<()> {
        self.plugin_manager.load_wasm(name, path, config).await
    }

    /// 并行批量加载 WASM 插件
    pub async fn load_wasm_plugins(&mut self, plugins: &[config::PluginConfig]) {
        let engine = self.plugin_manager.engine().clone();
        let tasks: Vec<_> = plugins
            .iter()
            .filter(|p| p.enabled)
            .map(|cfg| {
                let name = cfg.name.clone();
                let path = std::path::PathBuf::from(&cfg.path);
                let engine = engine.clone();
                async move {
                    let wasm_bytes = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
                    let instance = tokio::task::spawn_blocking(move || {
                        PluginInstance::load(engine, &wasm_bytes, serde_json::json!({}))
                    })
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e| e.to_string())?;
                    Ok::<_, String>((name, instance))
                }
            })
            .collect();

        let results = futures::future::join_all(tasks).await;
        for result in results {
            match result {
                Ok((name, instance)) => {
                    self.plugin_manager.register_wasm_instance(&name, Arc::new(instance));
                    tracing::info!("WASM 插件 {} 已加载", name);
                }
                Err(e) => {
                    tracing::error!("加载 WASM 插件失败: {}", e);
                }
            }
        }
    }

    /// 注册 Channel 绑定（测试注入用）
    pub fn bind_channel(&mut self, name: &str, channel_id: &str, plugins: Vec<String>) {
        self.bindings.add(name, channel_id.to_string(), plugins);
    }

    /// 启动事件循环
    pub async fn run(self) -> Result<()> {
        if self.bindings.is_empty() {
            tracing::info!("无频道注册，静默等待");
        }

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(1024);

        // 启动所有 Adapter
        for (name, adapter) in self.adapters.iter() {
            let tx = event_tx.clone();
            let shutdown = self.shutdown.clone();
            let adapter = adapter.clone();
            let name = name.to_owned();
            tokio::spawn(async move {
                if let Err(e) = adapter.start(name.clone(), tx, shutdown).await {
                    tracing::error!("适配器 {} 启动失败: {}", name, e);
                }
            });
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

                        let lookup = event.source_adapter.as_deref().unwrap_or(&event.platform);
                        let plugin_names = self.bindings.resolve(lookup, &event.channel_id);
                        tracing::debug!(
                            "channel 绑定解析: {}@{} → {} 个插件",
                            event.channel_id,
                            lookup,
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
                let lookup_key = target
                    .adapter_instance
                    .as_deref()
                    .unwrap_or(&target.platform);
                let span = tracing::info_span!(
                    "send_message",
                    instance = %lookup_key,
                    user_id = %target.user_id,
                    text = %content.text.chars().take(20).collect::<String>(),
                );
                async {
                    let adapter = self.adapters.get(lookup_key);
                    if let Some(adapter) = adapter {
                        if let Err(e) = adapter.send_message(&target, &content).await {
                            tracing::error!("发送消息失败: {}", e);
                        } else {
                            tracing::trace!("发送消息成功");
                        }
                    } else {
                        tracing::warn!("未找到适配器实例 {}", lookup_key);
                    }
                }
                .instrument(span)
                .await;
            }
            Action::Noop => {}
        }
    }
}
