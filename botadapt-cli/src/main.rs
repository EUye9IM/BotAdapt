use std::env;
use std::sync::Arc;
use std::time::Instant;

use botadapt_core::event::MessageContent;
use botadapt_core::plugin::native::{BuiltinCommand, BuiltinPlugin, CmdContext};
use botadapt_core::plugin::Action;
use botadapt_core::BotApp;
use botadapt_qq::adapter::QQAdapter;

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
    tracing_subscriber::fmt::init();

    let config_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "config/default.toml".into());

    tracing::info!("加载配置: {}", config_path);

    let config = match botadapt_core::config::Config::from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("配置加载失败: {}", e);
            return;
        }
    };

    let mut app = BotApp::from_config(config);

    // Phase 2: 根据配置动态加载 Adapter
    app.register_adapter(QQAdapter::new_arc());

    // 注册内置命令插件
    let ctx = Arc::new(CmdContext {
        start_time: Instant::now(),
    });
    let builtin = BuiltinPlugin::new("builtin", builtin_commands(), ctx);
    app.register_plugin(Box::new(builtin));

    if let Err(e) = app.run().await {
        tracing::error!("运行失败: {}", e);
    }
}
