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
