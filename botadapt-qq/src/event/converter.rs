use botadapt_core::event::{Event, EventKind, MessageContent, MessageEvent};
use uuid::Uuid;

use crate::api::types::C2cMessageData;

pub fn c2c_message_create(d: &serde_json::Value) -> Option<Event> {
    let data: C2cMessageData = serde_json::from_value(d.clone()).ok()?;
    let user_openid = &data.author.user_openid;

    Some(Event {
        id: Uuid::new_v4(),
        channel_id: format!("qq:c2c:{}", user_openid),
        platform: "qq".into(),
        timestamp: chrono_now_millis(),
        kind: EventKind::Message(MessageEvent {
            user_id: user_openid.clone(),
            group_id: None,
            channel_id: None,
            content: MessageContent {
                text: data.content,
                mentions: vec![],
                attachments: vec![],
            },
            raw: Some(d.clone()),
        }),
    })
}

fn chrono_now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use botadapt_core::event::EventKind;

    fn make_c2c_json(user_openid: &str, content: &str, msg_id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": msg_id,
            "author": { "user_openid": user_openid },
            "content": content,
            "timestamp": "2023-11-06T13:37:18+08:00"
        })
    }

    #[test]
    fn c2c_normal_message() {
        let d = make_c2c_json("USER_OPENID_ABC", "你好", "MSG_ID_001");
        let event = c2c_message_create(&d).expect("应成功转换");

        assert_eq!(event.platform, "qq");
        assert_eq!(event.channel_id, "qq:c2c:USER_OPENID_ABC");
        assert_eq!(event.timestamp, chrono_now_millis());
        assert!(event.timestamp > 0);

        match event.kind {
            EventKind::Message(msg) => {
                assert_eq!(msg.user_id, "USER_OPENID_ABC");
                assert_eq!(msg.group_id, None);
                assert_eq!(msg.channel_id, None);
                assert_eq!(msg.content.text, "你好");
                assert!(msg.content.mentions.is_empty());
                assert!(msg.content.attachments.is_empty());
                assert!(msg.raw.is_some());
            }
            _ => panic!("应为 Message 事件"),
        }
    }

    #[test]
    fn c2c_empty_content() {
        let d = make_c2c_json("U1", "", "MSG_002");
        let event = c2c_message_create(&d).expect("空内容也应转换");
        match event.kind {
            EventKind::Message(msg) => assert_eq!(msg.content.text, ""),
            _ => panic!("应为 Message 事件"),
        }
    }

    #[test]
    fn c2c_invalid_json_returns_none() {
        let d = serde_json::json!({ "not": "valid" });
        assert!(c2c_message_create(&d).is_none());
    }

    #[test]
    fn c2c_missing_author_returns_none() {
        let d = serde_json::json!({
            "id": "MSG_003",
            "content": "test",
            "timestamp": "2023-11-06T13:37:18+08:00"
        });
        assert!(c2c_message_create(&d).is_none());
    }
}
