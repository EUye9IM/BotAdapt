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
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            result = connect_and_dispatch(&api, &event_tx, &shutdown) => {
                match result {
                    Ok(()) => break,
                    Err(e) => {
                        tracing::warn!("WebSocket 断开: {}, 5秒后重连", e);
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
    let url = api.get_gateway_url().await?;
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

    // 1. 读取 Hello
    let hello = read_hello(&mut ws_read).await?;
    tracing::info!("收到 Hello, heartbeat_interval={}ms", hello.heartbeat_interval);

    // 2. 发送 Identify
    let id_data = IdentifyData {
        token: format!("QQBot {}", token),
        intents: INTENTS_C2C,
        shard: [0, 1],
        properties: None,
    };
    let id_payload = WsSend {
        op: 2,
        d: Some(serde_json::to_value(&id_data)?),
        s: None,
        t: None,
    };
    out_tx
        .send(serde_json::to_string(&id_payload)?)
        .map_err(|_| QqError::Connection("发送 Identify 失败".into()))?;

    // 3. 读取 Ready
    let ready = read_ready(&mut ws_read).await?;
    tracing::info!(
        "QQ 连接就绪, session_id={}, bot={}",
        ready.session_id,
        ready.user.username
    );

    // 4. 启动心跳
    let latest_seq = Arc::new(AtomicI64::new(0));
    let hb_tx = out_tx.clone();
    let hb_seq = latest_seq.clone();
    let hb_shutdown = shutdown.clone();
    tokio::spawn(async move {
        super::heartbeat::run(hb_tx, hb_seq, hello.heartbeat_interval, hb_shutdown).await;
    });

    // 5. 事件循环
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            msg = ws_read.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        let payload: WsPayload = serde_json::from_str(&text)?;
                        if let Some(s) = payload.s {
                            latest_seq.store(s, Ordering::SeqCst);
                        }

                        match payload.op {
                            0 => {
                                let t = payload.t.as_deref().unwrap_or("");
                                if t == "C2C_MESSAGE_CREATE" {
                                    if let Some(event) =
                                        crate::event::converter::c2c_message_create(&payload.d)
                                    {
                                        tracing::debug!("收到 C2C 消息: channel={}", event.channel_id);
                                        if event_tx.send(event).await.is_err() {
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                            7 | 9 => {
                                tracing::warn!("收到 opcode={}, 准备重连", payload.op);
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                    Some(Ok(WsMessage::Ping(_data))) => {
                        let _ = out_tx.send(serde_json::to_string(&serde_json::json!({
                            "op": 11
                        }))?);
                        // Also handle raw ping by sending pong through the same path
                        // Note: tokio-tungstenite should auto-pong, but handle explicitly
                    }
                    Some(Ok(WsMessage::Close(_))) | None => {
                        tracing::info!("WebSocket 连接关闭");
                        return Ok(());
                    }
                    Some(Err(e)) => {
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
