use serde::{Deserialize, Serialize};

use super::target::MessageTarget;
use super::event::MessageContent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    SendMessage {
        target: MessageTarget,
        content: MessageContent,
    },
    Noop,
}
