use std::env;
use std::sync::Arc;
use std::time::Instant;

use tracing_subscriber::fmt::format::FmtSpan;

use botadapt_core::event::MessageContent;
use botadapt_core::plugin::native::{BuiltinCommand, BuiltinPlugin, CmdContext};
use botadapt_core::plugin::Action;
use botadapt_core::BotApp;
use botadapt_qq::adapter::QQAdapter;
use botadapt_qq::config::QQConfig;

fn builtin_commands() -> Vec<BuiltinCommand> {
    vec![
        BuiltinCommand {
            name: "ping",
            description: "回复 pong",
            handler: Box::new(|_event, target, _args, _ctx| {
                Ok(vec![Action::SendMessage {
                    target: target.clone(),
                    content: MessageContent {
                        text: "pong!".into(),
                        mentions: vec![],
                        attachments: vec![],
                    },
                }])
            }),
        },
    ]
}

#[tokio::main]
async fn main() {
    let config_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "config/default.toml".into());

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
        if adapter_cfg.adapter_type == "qq" && adapter_cfg.enabled {
            if let Some(ref cfg) = adapter_cfg.config {
                match QQConfig::from_toml_value(cfg) {
                    Ok(qq_config) => {
                        app.register_adapter(QQAdapter::new_arc(qq_config));
                        tracing::info!("QQ 适配器已注册");
                    }
                    Err(e) => {
                        tracing::error!("QQ 适配器配置解析失败: {}", e);
                    }
                }
            } else {
                tracing::warn!("QQ 适配器缺少 config (app_id/client_secret)");
            }
        }
    }

    let ctx = Arc::new(CmdContext {
        start_time: Instant::now(),
    });
    let builtin = BuiltinPlugin::new("builtin", builtin_commands(), ctx);
    app.register_plugin(Box::new(builtin));

    if let Err(e) = app.run().await {
        tracing::error!("运行失败: {}", e);
    }
}
