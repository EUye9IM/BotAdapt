wit_bindgen::generate!({
    world: "tinybot-plugin",
    path: "../../wit/plugin.wit",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn msg_event(text: &str) -> BotEvent {
        BotEvent::Message(crate::tinybot::plugin::types::Message {
            target_type: "group".into(),
            target: "123".into(),
            content: MessageContent {
                text: text.into(),
            },
        })
    }

    #[test]
    fn active_returns_true_for_hello() {
        let evt = msg_event("hello");
        assert!(HelloPlugin::active(evt));
    }

    #[test]
    fn active_returns_true_for_hello_with_whitespace() {
        let evt = msg_event("  hello  ");
        assert!(HelloPlugin::active(evt));
    }

    #[test]
    fn active_returns_false_for_other_text() {
        let evt = msg_event("hi");
        assert!(!HelloPlugin::active(evt));
    }

    #[test]
    fn active_returns_false_for_empty() {
        let evt = msg_event("");
        assert!(!HelloPlugin::active(evt));
    }

    #[test]
    fn handle_with_empty_asks_name() {
        let evt = msg_event("");
        let action = HelloPlugin::handle(evt);
        assert!(!action.finish);
        assert!(action.reply.is_some());
        assert!(action.reply.unwrap().text.contains("名字"));
    }

    #[test]
    fn handle_with_text_greets() {
        let evt = msg_event("world");
        let action = HelloPlugin::handle(evt);
        assert!(action.finish);
        assert_eq!(action.reply.unwrap().text, "hello world");
    }
}
