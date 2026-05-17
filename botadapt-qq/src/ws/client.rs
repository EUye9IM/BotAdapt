use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_util::sync::CancellationToken;

use crate::api::types::{HelloData, IdentifyData, ReadyData, WsPayload, WsSend};
use crate::api::QqApi;
use crate::error::QqError;

const INTENTS_C2C: i64 = 1 << 25;

pub async fn run_loop(
    api: Arc<QqApi>,
    event_tx: mpsc::Sender<botadapt_core::event::Event>,
    shutdown: CancellationToken,
) {
    let mut retry_count = 0u32;
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            result = connect_and_dispatch(&api, &event_tx, &shutdown) => {
                match result {
                    Ok(()) => break,
                    Err(e) => {
                        retry_count += 1;
                        tracing::warn!(
                            retry_count,
                            "WebSocket 断开: {}, {}秒后重连 (第 {} 次)",
                            e,
                            5,
                            retry_count,
                        );
                    }
                }
                if shutdown.is_cancelled() {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn connect_and_dispatch(
    api: &QqApi,
    event_tx: &mpsc::Sender<botadapt_core::event::Event>,
    shutdown: &CancellationToken,
) -> Result<(), QqError> {
    let span = tracing::info_span!("ws_connect");
    let _ = span.enter();

    tracing::debug!("获取 Gateway 地址...");
    let url = api.get_gateway_url().await?;
    tracing::debug!(%url, "连接到 Gateway");

    let token = api.get_token().await?;

    let (ws, _) = connect_async(&url)
        .await
        .map_err(|e| QqError::Ws(e.to_string()))?;

    let (mut ws_write, mut ws_read) = ws.split();

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();

    let writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if ws_write.send(WsMessage::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let hello = read_hello(&mut ws_read).await?;
    tracing::info!(
        heartbeat_interval_ms = hello.heartbeat_interval,
        "收到 Hello"
    );

    let id_data = IdentifyData {
        token: format!("QQBot {}", token),
        intents: INTENTS_C2C,
        shard: [0, 1],
        properties: None,
    };
    tracing::debug!(
        intents = INTENTS_C2C,
        "发送 Identify"
    );
    let id_payload = WsSend {
        op: 2,
        d: Some(serde_json::to_value(&id_data)?),
        s: None,
        t: None,
    };
    out_tx
        .send(serde_json::to_string(&id_payload)?)
        .map_err(|_| QqError::Connection("发送 Identify 失败".into()))?;

    let ready = read_ready(&mut ws_read).await?;
    let session_id = ready.session_id.clone();
    let username = ready.user.username.clone();
    tracing::info!(
        %session_id,
        bot = %username,
        "QQ 连接就绪"
    );

    let latest_seq = Arc::new(AtomicI64::new(0));
    let hb_tx = out_tx.clone();
    let hb_seq = latest_seq.clone();
    let hb_shutdown = shutdown.clone();
    tokio::spawn(async move {
        super::heartbeat::run(hb_tx, hb_seq, hello.heartbeat_interval, hb_shutdown).await;
    });

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            msg = ws_read.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        let payload: WsPayload = serde_json::from_str(&text)?;
                        tracing::trace!(
                            op = payload.op,
                            s = payload.s,
                            t = payload.t.as_deref().unwrap_or("-"),
                            "ws_recv"
                        );
                        if let Some(s) = payload.s {
                            let prev = latest_seq.load(Ordering::SeqCst);
                            latest_seq.store(s, Ordering::SeqCst);
                            tracing::trace!(prev, new = s, "seq 更新");
                        }

                        match payload.op {
                            0 => {
                                let t = payload.t.as_deref().unwrap_or("");
                                if t == "C2C_MESSAGE_CREATE" {
                                    if let Some(event) =
                                        crate::event::converter::c2c_message_create(&payload.d)
                                    {
                                        tracing::debug!(
                                            channel_id = %event.channel_id,
                                            "收到 C2C 消息"
                                        );
                                        if event_tx.send(event).await.is_err() {
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                            7 | 9 => {
                                tracing::warn!(
                                    opcode = payload.op,
                                    "收到重连/无效会话信号"
                                );
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                    Some(Ok(WsMessage::Ping(_data))) => {
                        let _ = out_tx.send(serde_json::to_string(&serde_json::json!({
                            "op": 11
                        }))?);
                    }
                    Some(Ok(WsMessage::Close(_))) | None => {
                        tracing::info!("WebSocket 连接关闭");
                        return Ok(());
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket 读取错误: {}", e);
                        return Err(QqError::Ws(e.to_string()));
                    }
                    _ => {}
                }
            }
        }
    }

    drop(out_tx);
    let _ = writer.await;

    Ok(())
}

async fn read_hello(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) -> Result<HelloData, QqError> {
    loop {
        let msg = read
            .next()
            .await
            .ok_or_else(|| QqError::Connection("连接关闭, 未收到 Hello".into()))?
            .map_err(|e| QqError::Ws(e.to_string()))?;
        if let WsMessage::Text(text) = msg {
            let payload: WsPayload = serde_json::from_str(&text)?;
            if payload.op == 10 {
                return Ok(serde_json::from_value(payload.d)?);
            }
        }
    }
}

async fn read_ready(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) -> Result<ReadyData, QqError> {
    loop {
        let msg = read
            .next()
            .await
            .ok_or_else(|| QqError::Connection("连接关闭, 未收到 Ready".into()))?
            .map_err(|e| QqError::Ws(e.to_string()))?;
        if let WsMessage::Text(text) = msg {
            let payload: WsPayload = serde_json::from_str(&text)?;
            if payload.op == 0 && payload.t.as_deref() == Some("READY") {
                return Ok(serde_json::from_value(payload.d)?);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_payload(json: &str) -> WsPayload {
        serde_json::from_str(json).expect("WsPayload 解析失败")
    }

    #[test]
    fn dispatch_c2c_message_event() {
        let p = parse_payload(
            r#"{"op":0,"d":{"id":"X","content":"hi"},"s":5,"t":"C2C_MESSAGE_CREATE"}"#,
        );
        assert_eq!(p.op, 0);
        assert_eq!(p.t.as_deref(), Some("C2C_MESSAGE_CREATE"));
        assert_eq!(p.s, Some(5));
        assert!(!p.d.is_null());
    }

    #[test]
    fn dispatch_other_event_ignored() {
        let p = parse_payload(r#"{"op":0,"d":{},"s":1,"t":"GUILD_CREATE"}"#);
        assert_eq!(p.op, 0);
        assert_eq!(p.t.as_deref(), Some("GUILD_CREATE"));
    }

    #[test]
    fn opcode_reconnect_signals() {
        for op in [7, 9] {
            let json = format!(r#"{{"op":{}}}"#, op);
            let p: WsPayload = serde_json::from_str(&json)
                .unwrap_or_else(|_| panic!("op={} 解析失败", op));
            assert!(p.op == 7 || p.op == 9);
        }
    }

    #[test]
    fn heartbeat_ack() {
        let p = parse_payload(r#"{"op":11}"#);
        assert_eq!(p.op, 11);
        assert!(p.s.is_none());
        assert!(p.t.is_none());
    }

    #[test]
    fn payload_with_seq_updates() {
        let p = parse_payload(
            r#"{"op":0,"d":{"id":"X"},"s":100,"t":"C2C_MESSAGE_CREATE"}"#,
        );
        assert_eq!(p.s, Some(100));
    }

    #[test]
    fn payload_without_seq() {
        let p = parse_payload(r#"{"op":11}"#);
        assert_eq!(p.s, None);
    }

    #[test]
    fn payload_missing_optional_fields() {
        let p: WsPayload = serde_json::from_str(r#"{"op":0}"#).expect("最小 payload 应解析");
        assert_eq!(p.op, 0);
        assert!(p.d.is_null());
        assert_eq!(p.s, None);
        assert_eq!(p.t, None);
        assert_eq!(p.id, None);
    }
}
