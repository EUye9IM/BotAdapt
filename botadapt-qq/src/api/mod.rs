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
                if token.expires_at > Instant::now() {
                    return Ok(token.access_token.clone());
                }
            }
        }

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

        let access_token = token_resp.access_token.clone();
        let mut cached = self.cached_token.lock().await;
        *cached = Some(CachedToken {
            access_token: access_token.clone(),
            expires_at,
        });

        Ok(access_token)
    }

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
            return Err(QqError::Connection(format!(
                "Gateway 获取失败 HTTP {}: {}",
                status.as_u16(),
                &body_text[..body_text.len().min(200)]
            )));
        }

        let gw: GatewayResponse = serde_json::from_str(&body_text)?;
        Ok(gw.url)
    }

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

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(QqError::SendMessage(format!(
                "HTTP {}: {}",
                status.as_u16(),
                text
            )));
        }

        let _msg_resp = resp.json::<SendMessageResponse>().await?;
        Ok(())
    }
}
