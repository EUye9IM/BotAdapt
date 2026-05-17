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
    vec![BuiltinCommand {
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
    }]
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .init();

    let qq_config = QQConfig {
        app_id: env::var("QQ_APP_ID").expect("QQ_APP_ID 未设置"),
        client_secret: env::var("QQ_CLIENT_SECRET").expect("QQ_CLIENT_SECRET 未设置"),
    };

    let mut app = BotApp::empty();

    app.register_adapter(QQAdapter::new_arc(qq_config));

    let ctx = Arc::new(CmdContext {
        start_time: Instant::now(),
    });
    let builtin = BuiltinPlugin::new("builtin", builtin_commands(), ctx);
    app.register_plugin(Box::new(builtin));

    app.bind_channel("qq:c2c:*", vec!["builtin".into()]);
    app.bind_channel("qq:group:*", vec!["builtin".into()]);

    tracing::info!("QQ 测试程序启动，等待消息...");

    if let Err(e) = app.run().await {
        tracing::error!("运行失败: {}", e);
    }
}
