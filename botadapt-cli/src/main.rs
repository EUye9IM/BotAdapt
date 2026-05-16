use std::env;

use botadapt_core::BotApp;
use botadapt_qq::adapter::QQAdapter;

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

    if let Err(e) = app.run().await {
        tracing::error!("运行失败: {}", e);
    }
}
