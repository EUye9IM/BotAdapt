use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTarget {
    pub platform: String,
    pub user_id: String,
    pub group_id: Option<String>,
    pub channel_id: Option<String>,
    #[serde(default)]
    pub adapter_instance: Option<String>,
}
