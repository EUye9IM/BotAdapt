use clap::Parser;

mod args;
mod core;
mod platform;
#[tokio::main]
async fn main() {
    let arg = args::Args::parse();
    let config = match core::config::Config::from_file(&arg.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("配置加载失败: {}", e);
            return;
        }
    };
    let filter = tracing_subscriber::EnvFilter::new(&config.core.log_level);
    tracing_subscriber::fmt().with_env_filter(filter).init();

    tracing::info!("配置路径: {}", &arg.config);
    tracing::debug!("详细配置: {:?}", &config);
    let app = core::BotApp::from_config(&config);

    // let ctx = Arc::new(CmdContext {});
    // let builtin = BuiltinPlugin::new(builtin_commands(), ctx);
    // app.register_plugin("builtin", Box::new(builtin));

    // app.load_wasm_plugins(&config.plugins).await;

    if let Err(e) = app.run().await {
        tracing::error!("运行失败: {}", e);
    }
}
