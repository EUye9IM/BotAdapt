pub mod adapter;
pub mod binding;
pub mod config;
pub mod event;
pub mod plugin;

use std::sync::Arc;

use adapter::registry::AdapterRegistry;
use adapter::Adapter;
use binding::ChannelBinding;
use config::ChannelEntry;
use event::AdapterEvent;
use plugin::manager::PluginManager;
use plugin::wasm::PluginInstance;
use tokio::sync::mpsc;
use tracing::Instrument;

use crate::event::{AdapterEventWithName, MessageMeta, PluginEvent, PluginEventWithName};

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
    pub fn register_plugin(&mut self, name: &str, plugin: Box<dyn plugin::Plugin>) {
        self.plugin_manager.register(name, plugin);
    }

    /// 加载 WASM 插件
    pub async fn load_wasm_plugin(
        &mut self,
        name: &str,
        path: &std::path::Path,
        config: serde_json::Value,
    ) -> anyhow::Result<()> {
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
                    self.plugin_manager
                        .register_wasm_instance(&name, Arc::new(instance));
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
    pub async fn run(self) -> anyhow::Result<()> {
        if self.bindings.is_empty() {
            tracing::info!("无频道注册，静默等待");
        }

        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AdapterEventWithName>();

        // 启动所有 Adapter
        for (name, adapter) in self.adapters.iter() {
            let tx = event_tx.clone();
            let shutdown = self.shutdown.clone();
            let adapter = adapter.clone();
            let name = name.to_owned();
            let name2 = name.clone();
            let emit = Box::new(move |event: AdapterEvent| {
                if let Err(_e) = tx.send(AdapterEventWithName {
                    adapter_name: name.clone(),
                    event: event.clone(),
                }) {
                    tracing::error!("消息发送失败 {} {:?}", name, event)
                }
            });
            tokio::spawn(async move {
                if let Err(e) = adapter.start(emit, shutdown).await {
                    tracing::error!("适配器 {} 启动失败: {}", name2, e);
                }
            });
        }

        loop {
            tokio::select! {
                Some(event) = event_rx.recv() => {
                    let user_id = match &event.event {
                        AdapterEvent::Message(ref m) => match &m.meta {
                            MessageMeta::Private(ref p) => p.user_id.clone(),
                        },
                    };
                    let text_snippet = match &event.event {
                        AdapterEvent::Message(ref m) => {
                            m.content.text.chars().take(20).collect::<String>()
                        }
                    };
                    let span = tracing::info_span!(
                        "event",
                        adapter_id = %event.adapter_name,
                        user_id = %user_id,
                        text = %text_snippet,
                    );

                    async {
                        tracing::debug!("收到事件");
                        let adapt = event.adapter_name;
                        let AdapterEvent::Message(m) = event.event.clone();
                        let plugin_names = match m.meta {
                            MessageMeta::Private(p) => {
                                self.bindings.resolve(&adapt, &p.user_id)
                            }
                        };
                        tracing::debug!(
                            user_id = %user_id,
                            adapter = %adapt,
                            plugins = plugin_names.len(),
                            "channel 绑定解析"
                        );

                        if plugin_names.is_empty() {
                            tracing::debug!("channel 无绑定插件");
                            return;
                        }

                        let actions = self
                            .plugin_manager
                            .dispatch_parallel(&event.event, &plugin_names)
                            .await;

                        for action in actions {
                            self.execute_action(PluginEventWithName {
                                adapter_name: adapt.clone(),
                                event: action,
                            })
                            .await;
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
    async fn execute_action(&self, action: PluginEventWithName) {
        match action.event {
            PluginEvent::Message(e) => {
                let user_id = match &e.meta {
                    MessageMeta::Private(p) => p.user_id.clone(),
                };
                let text_snippet = e.content.text.chars().take(20).collect::<String>();
                let span = tracing::info_span!(
                    "send_message",
                    instance = %action.adapter_name,
                    user_id = %user_id,
                    text = %text_snippet,
                );
                async {
                    let adapter = self.adapters.get(&action.adapter_name);
                    if let Some(adapter) = adapter {
                        if let Err(e) = adapter.send_message(&e).await {
                            tracing::error!("发送消息失败: {}", e);
                        } else {
                            tracing::trace!("发送消息成功");
                        }
                    } else {
                        tracing::warn!("未找到适配器实例 {}", action.adapter_name);
                    }
                }
                .instrument(span)
                .await;
            }
        }
    }
}
