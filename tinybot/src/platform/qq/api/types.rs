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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_response_expires_in_string() {
        let json = r#"{"access_token":"ATOKEN","expires_in":"7200"}"#;
        let resp: AccessTokenResponse = serde_json::from_str(json).expect("字符串 expires_in 应解析");
        assert_eq!(resp.access_token, "ATOKEN");
        assert_eq!(resp.expires_in, 7200);
    }

    #[test]
    fn token_response_expires_in_number() {
        let json = r#"{"access_token":"ATOKEN","expires_in":3600}"#;
        let resp: AccessTokenResponse = serde_json::from_str(json).expect("数字 expires_in 应解析");
        assert_eq!(resp.access_token, "ATOKEN");
        assert_eq!(resp.expires_in, 3600);
    }

    #[test]
    fn ws_payload_dispatch() {
        let json = r#"{"op":0,"d":{"id":"X","content":"hi"},"s":42,"t":"C2C_MESSAGE_CREATE"}"#;
        let payload: WsPayload = serde_json::from_str(json).expect("Dispatch payload");
        assert_eq!(payload.op, 0);
        assert_eq!(payload.s, Some(42));
        assert_eq!(payload.t.as_deref(), Some("C2C_MESSAGE_CREATE"));
        assert!(!payload.d.is_null());
    }

    #[test]
    fn ws_payload_hello() {
        let json = r#"{"op":10,"d":{"heartbeat_interval":45000}}"#;
        let payload: WsPayload = serde_json::from_str(json).expect("Hello payload");
        assert_eq!(payload.op, 10);
        assert!(payload.s.is_none());
        assert!(payload.t.is_none());

        let hello: HelloData = serde_json::from_value(payload.d).expect("HelloData");
        assert_eq!(hello.heartbeat_interval, 45000);
    }

    #[test]
    fn ws_payload_heartbeat_ack() {
        let json = r#"{"op":11}"#;
        let payload: WsPayload = serde_json::from_str(json).expect("Heartbeat ACK payload");
        assert_eq!(payload.op, 11);
    }

    #[test]
    fn ws_send_heartbeat() {
        let ws_send = WsSend {
            op: 1,
            d: Some(serde_json::json!({"d": 42})),
            s: None,
            t: None,
        };
        let json = serde_json::to_string(&ws_send).expect("序列化心跳");
        assert!(json.contains("\"op\":1"));
        assert!(json.contains("\"d\""));
        assert!(!json.contains("\"s\""));
        assert!(!json.contains("\"t\""));
    }

    #[test]
    fn ws_send_identify() {
        let id_data = IdentifyData {
            token: "QQBot TOKEN".into(),
            intents: 33554432,
            shard: [0, 1],
            properties: None,
        };
        let ws_send = WsSend {
            op: 2,
            d: Some(serde_json::to_value(&id_data).unwrap()),
            s: None,
            t: None,
        };
        let json = serde_json::to_string(&ws_send).expect("序列化 Identify");
        assert!(json.contains("\"op\":2"));
        assert!(json.contains("\"token\":\"QQBot TOKEN\""));
        assert!(json.contains("\"intents\":33554432"));
        assert!(!json.contains("\"properties\""));
    }

    #[test]
    fn c2c_message_data_deserialize() {
        let json = r#"{
            "id":"ROBOT1.0_xxx",
            "author":{"user_openid":"OPENID123"},
            "content":"你好世界",
            "timestamp":"2023-11-06T13:37:18+08:00"
        }"#;
        let data: C2cMessageData = serde_json::from_str(json).expect("C2C 消息反序列化");
        assert_eq!(data.id, "ROBOT1.0_xxx");
        assert_eq!(data.author.user_openid, "OPENID123");
        assert_eq!(data.content, "你好世界");
    }

    #[test]
    fn ready_data_deserialize() {
        let json = r#"{
            "version":1,
            "session_id":"sess-001",
            "user":{"id":"bot123","username":"测试机器人","bot":true},
            "shard":[0,0]
        }"#;
        let ready: ReadyData = serde_json::from_str(json).expect("Ready 反序列化");
        assert_eq!(ready.session_id, "sess-001");
        assert_eq!(ready.user.id, "bot123");
        assert_eq!(ready.user.username, "测试机器人");
        assert!(ready.user.bot);
    }
}
