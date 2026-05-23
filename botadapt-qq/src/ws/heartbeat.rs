use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub async fn run(
    ws_tx: mpsc::UnboundedSender<String>,
    latest_seq: Arc<AtomicI64>,
    interval_ms: u64,
    shutdown: CancellationToken,
    mut ack_rx: mpsc::UnboundedReceiver<()>,
    timeout_tx: mpsc::UnboundedSender<()>,
) {
    let interval = Duration::from_millis(interval_ms);
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = tokio::time::sleep(interval) => {
                while ack_rx.try_recv().is_ok() {}

                let seq = latest_seq.load(Ordering::SeqCst);
                tracing::trace!(seq, interval_ms, "心跳发送");
                let payload = serde_json::json!({
                    "op": 1,
                    "d": seq
                });
                if ws_tx.send(payload.to_string()).is_err() {
                    break;
                }

                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_secs(3)) => {
                        tracing::warn!("心跳超时: 3秒内未收到 OpCode 11 响应");
                        let _ = timeout_tx.send(());
                        break;
                    }
                    ack = ack_rx.recv() => {
                        if ack.is_none() {
                            break;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI64;

    #[tokio::test]
    async fn heartbeat_sends_at_interval() {
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let seq = Arc::new(AtomicI64::new(42));
        let shutdown = CancellationToken::new();
        let (ack_tx, ack_rx) = mpsc::unbounded_channel::<()>();
        let (hb_err_tx, _hb_err_rx) = mpsc::unbounded_channel::<()>();

        let h_seq = seq.clone();
        let h_shutdown = shutdown.clone();
        let handle = tokio::spawn(async move {
            run(tx, h_seq, 50, h_shutdown, ack_rx, hb_err_tx).await;
        });

        let msg = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
            .await
            .expect("超时")
            .expect("channel 已关闭");

        let parsed: serde_json::Value = serde_json::from_str(&msg).expect("心跳应为 JSON");
        assert_eq!(parsed["op"], 1);
        assert_eq!(parsed["d"], 42);

        let _ = ack_tx.send(());

        seq.store(99, Ordering::SeqCst);
        let msg2 = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
            .await
            .expect("超时")
            .expect("channel 已关闭");

        let parsed2: serde_json::Value = serde_json::from_str(&msg2).unwrap();
        assert_eq!(parsed2["op"], 1);
        assert_eq!(parsed2["d"], 99);

        shutdown.cancel();
        handle.await.expect("heartbeat task 应正常退出");
    }

    #[tokio::test]
    async fn heartbeat_stops_on_shutdown() {
        let (tx, _rx) = mpsc::unbounded_channel::<String>();
        let seq = Arc::new(AtomicI64::new(0));
        let shutdown = CancellationToken::new();
        let (_ack_tx, ack_rx) = mpsc::unbounded_channel::<()>();
        let (hb_err_tx, _hb_err_rx) = mpsc::unbounded_channel::<()>();

        let h_shutdown = shutdown.clone();
        let handle = tokio::spawn(async move {
            run(tx, seq, 1000, h_shutdown, ack_rx, hb_err_tx).await;
        });

        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(1), handle)
            .await
            .expect("超时")
            .expect("heartbeat task 应正常退出");
    }

    #[tokio::test]
    async fn heartbeat_timeout_triggers_on_no_ack() {
        let (tx, _rx) = mpsc::unbounded_channel::<String>();
        let seq = Arc::new(AtomicI64::new(0));
        let shutdown = CancellationToken::new();
        let (_ack_tx, ack_rx) = mpsc::unbounded_channel::<()>();
        let (hb_err_tx, mut hb_err_rx) = mpsc::unbounded_channel::<()>();

        let h_shutdown = shutdown.clone();
        let handle = tokio::spawn(async move {
            run(tx, seq, 10, h_shutdown, ack_rx, hb_err_tx).await;
        });

        let timeout_signal =
            tokio::time::timeout(std::time::Duration::from_secs(5), hb_err_rx.recv())
                .await
                .expect("应在 5 秒内收到超时信号")
                .expect("hb_err channel 不应关闭");

        assert_eq!(timeout_signal, ());

        handle.await.expect("heartbeat task 应正常退出");
    }
}
