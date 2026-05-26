use crate::core::{bot::Bot, config::BotConfig};
use anyhow::Result;
use std::sync::Arc;
mod debug;
pub fn new_bot_from_config(cfg: &BotConfig) -> Result<Arc<dyn Bot>> {
    match cfg.platform.as_str() {
        debug::PLAT => return Ok(Arc::new(debug::Bot::new(&cfg.config)?)),
        other => {
            anyhow::bail!("未知的平台：{}", other)
        }
    }
}
