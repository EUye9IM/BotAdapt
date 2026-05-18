use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub channel_id: String,
    pub platform: String,
    pub timestamp: i64,
    pub kind: EventKind,
    #[serde(default)]
    pub source_adapter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
    Message(MessageEvent),
    Notice(NoticeEvent),
    Request(RequestEvent),
    Meta(MetaEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEvent {
    pub user_id: String,
    pub group_id: Option<String>,
    pub channel_id: Option<String>,
    pub content: MessageContent,
    pub raw: Option<serde_json::Value>,
}

impl MessageEvent {
    pub fn reply_target(&self, platform: &str) -> MessageTarget {
        MessageTarget {
            platform: platform.to_string(),
            user_id: self.user_id.clone(),
            group_id: self.group_id.clone(),
            channel_id: self.channel_id.clone(),
            adapter_instance: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoticeEvent {
    pub notice_type: String,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub raw: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestEvent {
    pub request_type: String,
    pub user_id: String,
    pub group_id: Option<String>,
    pub raw: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaEvent {
    pub meta_type: String,
    pub raw: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub text: String,
    #[serde(default)]
    pub mentions: Vec<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub url: String,
    pub filename: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTarget {
    pub platform: String,
    pub user_id: String,
    pub group_id: Option<String>,
    pub channel_id: Option<String>,
    #[serde(default)]
    pub adapter_instance: Option<String>,
}
