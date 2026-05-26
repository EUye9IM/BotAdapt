use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterEventWithName {
    pub adapter_name: String,
    pub event: BotEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEventWithName {
    pub adapter_name: String,
    pub event: PluginEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BotEvent {
    Message(Message),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    Message(Message),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub target_type: String,
    pub target: String,
    pub content: MessageContent,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub text: String,
}
