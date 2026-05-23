use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdapterEvent {
    Message(MessageEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    Message(MessageEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEvent {
    pub meta: MessageMeta,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageMeta {
    Private(PrivateMeta),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateMeta {
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub text: String,
}
