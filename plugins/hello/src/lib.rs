wit_bindgen::generate!({
    world: "tinybot-plugin",
    path: "wit/world.wit",
});

use crate::exports::tinybot::plugin::plugin_api::{Action, BotEvent, Guest};
use crate::tinybot::plugin::types::MessageContent;

struct HelloPlugin;

impl Guest for HelloPlugin {
    fn active(evt: BotEvent) -> bool {
        matches!(evt, BotEvent::Message(ref m) if m.content.text.trim() == "hello")
    }

    fn handle(evt: BotEvent) -> Action {
        let BotEvent::Message(m) = evt;
        let text = m.content.text.trim().to_string();
        if text.is_empty() {
            Action {
                finish: false,
                reply: Some(MessageContent {
                    text: "你的名字是什么?".to_string(),
                }),
            }
        } else {
            Action {
                finish: true,
                reply: Some(MessageContent {
                    text: format!("hello {}", text),
                }),
            }
        }
    }
}

export!(HelloPlugin);
