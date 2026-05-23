use std::sync::Arc;
use std::{env, vec};

use tracing_subscriber::fmt::format::FmtSpan;

use botadapt_core::event::{
    AdapterEvent, MessageContent, MessageEvent, MessageMeta, PluginEvent, PrivateMeta,
};
use botadapt_core::plugin::native::{BuiltinCommand, BuiltinPlugin, CmdContext};
use botadapt_core::BotApp;
use botadapt_qq::adapter::QQAdapter;
use botadapt_qq::PLATFORM_ID;

fn builtin_commands() -> Vec<BuiltinCommand> {
    vec![BuiltinCommand {
        name: "ping",
        description: "回复 pong",
        handler: Box::new(|event, _ctx| {
            tracing::debug!("receive {:?}", event);
            let AdapterEvent::Message(m) = event;
            let MessageMeta::Private(p) = m.meta.clone();
            Ok(vec![PluginEvent::Message(MessageEvent {
                meta: MessageMeta::Private(PrivateMeta { user_id: p.user_id }),
                content: MessageContent {
                    text: "pong!".to_owned(),
                },
            })])
        }),
    }]
}

#[tokio::main]
async fn main() {
    let config_path = env::args().nth(1).unwrap_or_else(|| "botadapt.toml".into());

    let config = match botadapt_core::config::Config::from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("配置加载失败: {}", e);
            return;
        }
    };

    let filter = tracing_subscriber::EnvFilter::new(&config.core.log_level);
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .init();

    tracing::info!("加载配置: {}", config_path);

    let mut app = BotApp::from_config(config.clone());

    for adapter_cfg in &config.adapters {
        if !adapter_cfg.enabled {
            continue;
        }
        match adapter_cfg.adapter_type.as_str() {
            PLATFORM_ID => match QQAdapter::new(&adapter_cfg.config) {
                Ok(adapter) => {
                    app.register_adapter(
                        &adapter_cfg.name,
                        Arc::new(adapter),
                        &adapter_cfg.channels,
                    );
                    tracing::info!(name = adapter_cfg.name, "QQ 适配器已注册");
                }
                Err(e) => {
                    tracing::error!("QQ 适配器创建失败: {}", e);
                }
            },
            name => {
                tracing::error!("未知的适配器: {}", name);
            }
        }
    }

    let ctx = Arc::new(CmdContext {});
    let builtin = BuiltinPlugin::new(builtin_commands(), ctx);
    app.register_plugin("builtin", Box::new(builtin));

    app.load_wasm_plugins(&config.plugins).await;

    if let Err(e) = app.run().await {
        tracing::error!("运行失败: {}", e);
    }
}
