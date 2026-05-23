use botadapt_core::event::{AdapterEvent, MessageContent, MessageEvent, MessageMeta, PrivateMeta};

use crate::api::types::C2cMessageData;

pub fn c2c_message_create(d: &serde_json::Value) -> Option<AdapterEvent> {
    tracing::trace!(payload = %d, "开始转换 C2C 消息");

    let data: C2cMessageData = serde_json::from_value(d.clone()).ok()?;
    let user_openid = &data.author.user_openid;

    let event = AdapterEvent::Message(MessageEvent {
        meta: MessageMeta::Private(PrivateMeta {
            user_id: user_openid.clone(),
        }),
        content: MessageContent {
            text: data.content.clone(),
        },
    });

    tracing::debug!(
        user_id = %user_openid,
        content = %data.content.chars().take(30).collect::<String>(),
        "C2C 消息已转换"
    );

    Some(event)
}

#[cfg(test)]
mod tests {
    use super::*;

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

        match event {
            AdapterEvent::Message(msg) => {
                assert_eq!(msg.content.text, "你好");
                match msg.meta {
                    MessageMeta::Private(p) => {
                        assert_eq!(p.user_id, "USER_OPENID_ABC");
                    }
                }
            }
        }
    }

    #[test]
    fn c2c_empty_content() {
        let d = make_c2c_json("U1", "", "MSG_002");
        let event = c2c_message_create(&d).expect("空内容也应转换");

        match event {
            AdapterEvent::Message(msg) => {
                assert_eq!(msg.content.text, "");
                match msg.meta {
                    MessageMeta::Private(p) => {
                        assert_eq!(p.user_id, "U1");
                    }
                }
            }
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
