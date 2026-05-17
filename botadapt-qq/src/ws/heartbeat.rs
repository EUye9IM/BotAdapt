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
) {
    let interval = Duration::from_millis(interval_ms);
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = tokio::time::sleep(interval) => {
                let seq = latest_seq.load(Ordering::SeqCst);
                let payload = serde_json::json!({
                    "op": 1,
                    "d": seq
                });
                if ws_tx.send(payload.to_string()).is_err() {
                    break;
                }
            }
        }
    }
}
