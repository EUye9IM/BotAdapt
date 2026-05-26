pub mod types;

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use self::types::{AccessTokenRequest, AccessTokenResponse, GatewayResponse, SendMessageRequest, SendMessageResponse};
use crate::config::QQConfig;
use crate::error::QqError;

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

pub struct QqApi {
    http: reqwest::Client,
    app_id: String,
    client_secret: String,
    cached_token: Mutex<Option<CachedToken>>,
}

const TOKEN_URL: &str = "https://bots.qq.com/app/getAppAccessToken";
const GATEWAY_URL: &str = "https://api.sgroup.qq.com/gateway";
const API_BASE: &str = "https://api.sgroup.qq.com";
const REFRESH_BEFORE: Duration = Duration::from_secs(60);

impl QqApi {
    pub fn new(config: &QQConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            app_id: config.app_id.clone(),
            client_secret: config.client_secret.clone(),
            cached_token: Mutex::new(None),
        }
    }

    pub fn new_arc(config: &QQConfig) -> Arc<Self> {
        Arc::new(Self::new(config))
    }

    pub async fn get_token(&self) -> Result<String, QqError> {
        {
            let cached = self.cached_token.lock().await;
            if let Some(ref token) = *cached {
                let remaining = token.expires_at.saturating_duration_since(Instant::now());
                if remaining > Duration::ZERO {
                    tracing::debug!(
                        remaining_secs = remaining.as_secs(),
                        "使用缓存的 AccessToken"
                    );
                    return Ok(token.access_token.clone());
                }
            }
        }

        tracing::debug!("获取新的 AccessToken");
        let req = AccessTokenRequest {
            app_id: self.app_id.clone(),
            client_secret: self.client_secret.clone(),
        };

        let resp = self
            .http
            .post(TOKEN_URL)
            .json(&req)
            .send()
            .await?;

        let status = resp.status();
        let body_text = resp.text().await?;

        if !status.is_success() {
            tracing::error!(%status, body = %&body_text[..body_text.len().min(200)], "AccessToken 获取失败");
            return Err(QqError::Auth(format!(
                "Token 获取失败 HTTP {}: {}",
                status.as_u16(),
                &body_text[..body_text.len().min(200)]
            )));
        }

        let token_resp: AccessTokenResponse = serde_json::from_str(&body_text)?;

        let expires_at = Instant::now()
            + Duration::from_secs(token_resp.expires_in as u64)
            - REFRESH_BEFORE;

        tracing::debug!(
            expires_in = token_resp.expires_in,
            "AccessToken 已获取"
        );

        let access_token = token_resp.access_token.clone();
        let mut cached = self.cached_token.lock().await;
        *cached = Some(CachedToken {
            access_token: access_token.clone(),
            expires_at,
        });

        Ok(access_token)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_gateway_url(&self) -> Result<String, QqError> {
        let token = self.get_token().await?;
        let resp = self
            .http
            .get(GATEWAY_URL)
            .header("Authorization", format!("QQBot {}", token))
            .send()
            .await?;

        let status = resp.status();
        let body_text = resp.text().await?;

        if !status.is_success() {
            tracing::error!(%status, "Gateway 获取失败");
            return Err(QqError::Connection(format!(
                "Gateway 获取失败 HTTP {}: {}",
                status.as_u16(),
                &body_text[..body_text.len().min(200)]
            )));
        }

        let gw: GatewayResponse = serde_json::from_str(&body_text)?;
        tracing::debug!(url = %gw.url, "Gateway 地址已获取");
        Ok(gw.url)
    }

    #[tracing::instrument(skip(self), fields(openid = %openid, text = %content.chars().take(20).collect::<String>()))]
    pub async fn send_c2c_message(
        &self,
        openid: &str,
        content: &str,
        msg_id: Option<&str>,
    ) -> Result<(), QqError> {
        let token = self.get_token().await?;
        let url = format!("{}/v2/users/{}/messages", API_BASE, openid);

        let body = SendMessageRequest {
            content: content.to_string(),
            msg_type: 0,
            msg_id: msg_id.map(|s| s.to_string()),
            msg_seq: None,
        };

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("QQBot {}", token))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            tracing::error!(%status, body = %&text[..text.len().min(200)], "发送消息失败");
            return Err(QqError::SendMessage(format!(
                "HTTP {}: {}",
                status.as_u16(),
                text
            )));
        }

        let _msg_resp = resp.json::<SendMessageResponse>().await?;
        tracing::debug!(%status, "发送消息成功");
        Ok(())
    }
}
