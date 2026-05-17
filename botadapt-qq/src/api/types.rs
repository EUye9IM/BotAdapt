use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct AccessTokenRequest {
    #[serde(rename = "appId")]
    pub app_id: String,
    #[serde(rename = "clientSecret")]
    pub client_secret: String,
}

#[derive(Deserialize)]
pub struct AccessTokenResponse {
    pub access_token: String,
    #[serde(deserialize_with = "deserialize_expires_in")]
    pub expires_in: i64,
}

fn deserialize_expires_in<'de, D>(d: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;
    struct ExpiresVisitor;
    impl de::Visitor<'_> for ExpiresVisitor {
        type Value = i64;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a number or string representing seconds")
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<i64, E> {
            Ok(v)
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<i64, E> {
            Ok(v as i64)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<i64, E> {
            v.parse::<i64>().map_err(de::Error::custom)
        }
    }
    d.deserialize_any(ExpiresVisitor)
}

#[derive(Deserialize)]
pub struct GatewayResponse {
    pub url: String,
}

#[derive(Serialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub msg_type: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg_seq: Option<i32>,
}

#[derive(Deserialize)]
pub struct SendMessageResponse {
    pub id: String,
    pub timestamp: String,
}

// WebSocket Payload

#[derive(Deserialize, Debug)]
pub struct WsPayload {
    pub op: i32,
    #[serde(default)]
    pub d: serde_json::Value,
    #[serde(default)]
    pub s: Option<i64>,
    #[serde(default)]
    pub t: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Serialize)]
pub struct WsSend {
    pub op: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub d: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<String>,
}

// OpCode 10 Hello

#[derive(Deserialize, Debug)]
pub struct HelloData {
    pub heartbeat_interval: u64,
}

// OpCode 2 Identify

#[derive(Serialize)]
pub struct IdentifyData {
    pub token: String,
    pub intents: i64,
    pub shard: [i32; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Value>,
}

// OpCode 0 READY

#[derive(Deserialize, Debug)]
pub struct ReadyData {
    pub version: i32,
    pub session_id: String,
    pub user: ReadyUser,
    pub shard: Option<Vec<i32>>,
}

#[derive(Deserialize, Debug)]
pub struct ReadyUser {
    pub id: String,
    pub username: String,
    pub bot: bool,
}

// OpCode 0 C2C_MESSAGE_CREATE

#[derive(Deserialize, Debug)]
pub struct C2cMessageData {
    pub id: String,
    pub author: C2cAuthor,
    pub content: String,
    pub timestamp: String,
}

#[derive(Deserialize, Debug)]
pub struct C2cAuthor {
    pub user_openid: String,
}
