use crate::core::{
    events::{BotEvent, MessageContent},
    plugin::{Action, Plugin, PluginFactory, PluginManager},
};

struct PingPlugin;

struct PingPluginFactory;

impl PluginFactory for PingPluginFactory {
    fn name(&self) -> &str {
        "ping"
    }

    fn active(&self, evt: &BotEvent) -> bool {
        match evt {
            BotEvent::Message(m) => m.content.text.trim() == "/ping",
        }
    }

    fn create(&self) -> anyhow::Result<Box<dyn Plugin>> {
        Ok(Box::new(PingPlugin))
    }
}

impl Plugin for PingPlugin {
    fn handle(&mut self, _evt: &BotEvent) -> anyhow::Result<Action> {
        Ok(Action {
            finish: true,
            reply: MessageContent {
                text: "pong!".to_owned(),
            },
        })
    }
}

/// 注册所有内置插件到 PluginManager
pub fn register_builtins(mgr: &mut PluginManager) {
    mgr.register(Box::new(PingPluginFactory));
}
