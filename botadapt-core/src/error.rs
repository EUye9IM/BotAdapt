use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML 解析错误: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("适配器错误: {0}")]
    Adapter(String),

    #[error("插件错误: {0}")]
    Plugin(String),

    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
