use thiserror::Error;

#[derive(Debug, Error)]
pub enum QqError {
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),

    #[error("WebSocket 错误: {0}")]
    Ws(String),

    #[error("JSON 解析错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("鉴权失败: {0}")]
    Auth(String),

    #[error("连接错误: {0}")]
    Connection(String),

    #[error("发送消息失败: {0}")]
    SendMessage(String),

    #[error("配置解析失败: {0}")]
    Config(#[from] toml::de::Error),
}
