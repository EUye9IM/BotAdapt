use crate::core::{
    events::{BotEvent, Message, MessageContent},
    plugin::{Action, Plugin, PluginFactory, PluginManager},
};

struct PingPlugin {
    i: bool,
}

struct PingPluginFactory;

impl PluginFactory for PingPluginFactory {
    fn active(&self, evt: &BotEvent) -> bool {
        match evt {
            BotEvent::Message(m) => m.content.text.trim() == "hello",
        }
    }

    fn create(&self) -> anyhow::Result<Box<dyn Plugin>> {
        Ok(Box::new(PingPlugin { i: false }))
    }
}

impl Plugin for PingPlugin {
    fn handle(&mut self, evt: &BotEvent) -> anyhow::Result<Action> {
        if self.i
            && let BotEvent::Message(m) = evt
        {
            Ok(Action {
                finish: true,
                reply: Some(MessageContent {
                    text: format!("hello {}", m.content.text),
                }),
            })
        } else {
            self.i = true;
            Ok(Action {
                finish: false,
                reply: Some(MessageContent {
                    text: "hi!".to_owned(),
                }),
            })
        }
    }
}

/// 注册所有内置插件到 PluginManager
pub fn register_builtins(mgr: &mut PluginManager) {
    mgr.register("hello", Box::new(PingPluginFactory));
}
