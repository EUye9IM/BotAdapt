use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;

use crate::error::Result;
use crate::event::{Event, EventKind, MessageTarget};
use crate::plugin::{Action, Plugin};

/// 命令间共享的上下文（启动时间等）
pub struct CmdContext {
    pub start_time: Instant,
}

/// 命令处理函数签名：接收 Event（含平台信息）、回复目标、参数文本、共享上下文
pub type CmdHandler =
    Box<dyn Fn(&Event, &MessageTarget, &str, &CmdContext) -> Result<Vec<Action>> + Send + Sync>;

/// 单个内置命令
pub struct BuiltinCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub handler: CmdHandler,
}

/// 内置命令插件。扫描消息中 `/` 开头的命令，匹配后调用对应 handler。
pub struct BuiltinPlugin {
    commands: HashMap<String, BuiltinCommand>,
    ctx: Arc<CmdContext>,
}

impl BuiltinPlugin {
    pub fn new(commands: Vec<BuiltinCommand>, ctx: Arc<CmdContext>) -> Self {
        let mut cmd_map = HashMap::new();
        for cmd in commands {
            cmd_map.insert(format!("/{}", cmd.name), cmd);
        }
        Self {
            commands: cmd_map,
            ctx,
        }
    }
}

#[async_trait]
impl Plugin for BuiltinPlugin {
    async fn handle_event(&self, event: Event) -> Result<Vec<Action>> {
        let (target, text) = match &event.kind {
            EventKind::Message(msg) => {
                let target = MessageTarget {
                    platform: event.platform.clone(),
                    user_id: msg.user_id.clone(),
                    group_id: msg.group_id.clone(),
                    channel_id: msg.channel_id.clone(),
                    adapter_instance: event.source_adapter.clone(),
                };
                (target, msg.content.text.clone())
            }
            _ => return Ok(vec![]),
        };

        if let Some((cmd_name, args)) = split_cmd(&text) {
            if let Some(cmd) = self.commands.get(cmd_name) {
                return (cmd.handler)(&event, &target, args, &self.ctx);
            }
        }

        Ok(vec![])
    }
}

/// 从消息文本中拆分命令名和参数。
/// 例如 "/ping" → Some(("/ping", ""))
///     "/echo hello world" → Some(("/echo", "hello world"))
///     "普通消息" → None
fn split_cmd(text: &str) -> Option<(&str, &str)> {
    let text = text.trim();
    if !text.starts_with('/') {
        return None;
    }
    if let Some(space_pos) = text.find(' ') {
        let cmd = &text[..space_pos];
        let args = text[space_pos..].trim();
        Some((cmd, args))
    } else {
        Some((text, ""))
    }
}
