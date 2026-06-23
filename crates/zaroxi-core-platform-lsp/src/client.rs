//! Asynchronous JSON-RPC LSP client with request/response correlation and
//! built-in round-trip latency instrumentation ([`crate::trace`]).
//!
//! The client is generic over any `AsyncRead`/`AsyncWrite` pair, so it drives a
//! real language-server subprocess (`ChildStdout`/`ChildStdin`) in production
//! and an in-memory `tokio::io::duplex` fake server in tests. Two background
//! tasks own the wire: a reader that frames incoming messages and routes
//! responses to per-request `oneshot` channels (notifications go to an `mpsc`),
//! and a writer fed by an `mpsc` so request dispatch never holds a lock across
//! an `.await`.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

use crate::trace::{LspMethod, LspTiming};

/// Error from the LSP client (transport closed, protocol error, or a JSON-RPC
/// `error` object returned by the server).
#[derive(Debug, Clone)]
pub struct LspError(pub String);

impl std::fmt::Display for LspError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for LspError {}

type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>;

/// A connected JSON-RPC LSP client.
pub struct LspClient {
    next_id: AtomicU64,
    pending: PendingMap,
    writer_tx: mpsc::UnboundedSender<Vec<u8>>,
    notif_rx: Mutex<Option<mpsc::UnboundedReceiver<(String, Value)>>>,
}

impl LspClient {
    /// Build a client over `reader`/`writer`, spawning the reader and writer
    /// background tasks. Use `ChildStdout`/`ChildStdin` for a real server or
    /// `tokio::io::duplex` halves for tests.
    pub fn new<R, W>(reader: R, writer: W) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (writer_tx, writer_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (notif_tx, notif_rx) = mpsc::unbounded_channel::<(String, Value)>();

        tokio::spawn(reader_loop(reader, pending.clone(), notif_tx));
        tokio::spawn(writer_loop(writer, writer_rx));

        Self {
            next_id: AtomicU64::new(1),
            pending,
            writer_tx,
            notif_rx: Mutex::new(Some(notif_rx)),
        }
    }

    /// Take the server-notification receiver (e.g. `publishDiagnostics`). Yields
    /// `Some` exactly once; subsequent calls return `None`.
    pub fn take_notifications(&self) -> Option<mpsc::UnboundedReceiver<(String, Value)>> {
        self.notif_rx.lock().unwrap().take()
    }

    /// Frame a JSON value as an LSP message (`Content-Length` header + body).
    fn frame(value: &Value) -> Vec<u8> {
        let body = serde_json::to_vec(value).unwrap_or_default();
        let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        out.extend_from_slice(&body);
        out
    }

    fn register(&self) -> (u64, oneshot::Receiver<Value>) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);
        (id, rx)
    }

    fn parse_result(resp: Value) -> Result<Value, LspError> {
        if let Some(err) = resp.get("error") {
            return Err(LspError(err.to_string()));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Send a JSON-RPC request and await its correlated response.
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, LspError> {
        let (id, rx) = self.register();
        let msg = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.writer_tx
            .send(Self::frame(&msg))
            .map_err(|_| LspError("lsp writer task closed".to_string()))?;
        let resp = rx.await.map_err(|_| LspError("lsp response channel dropped".to_string()))?;
        Self::parse_result(resp)
    }

    /// Send a request and measure round-trip latency.
    ///
    /// Returns the result plus an [`LspTiming`] with `request_sent_ms` (build +
    /// enqueue) and `response_recv_ms` (dispatch → response) populated;
    /// `apply_ms` is left at `0.0` for the caller to fill after applying the
    /// result. Pass the JSON-RPC method via [`LspMethod::rpc_method`].
    pub async fn request_timed(
        &self,
        method: LspMethod,
        params: Value,
    ) -> Result<(Value, LspTiming), LspError> {
        use tokio::time::Instant;

        let mut timing = LspTiming::new(method);

        let t_build = Instant::now();
        let (id, rx) = self.register();
        let msg =
            json!({"jsonrpc": "2.0", "id": id, "method": method.rpc_method(), "params": params});
        self.writer_tx
            .send(Self::frame(&msg))
            .map_err(|_| LspError("lsp writer task closed".to_string()))?;
        timing.request_sent_ms = t_build.elapsed().as_secs_f32() * 1000.0;

        let t_resp = Instant::now();
        let resp = rx.await.map_err(|_| LspError("lsp response channel dropped".to_string()))?;
        timing.response_recv_ms = t_resp.elapsed().as_secs_f32() * 1000.0;

        let result = Self::parse_result(resp)?;
        Ok((result, timing))
    }

    /// `textDocument/completion`, timed. Convenience wrapper over
    /// [`Self::request_timed`].
    pub async fn completion(&self, params: Value) -> Result<(Value, LspTiming), LspError> {
        self.request_timed(LspMethod::Completion, params).await
    }

    /// `textDocument/hover`, timed.
    pub async fn hover(&self, params: Value) -> Result<(Value, LspTiming), LspError> {
        self.request_timed(LspMethod::Hover, params).await
    }

    /// `textDocument/definition`, timed.
    pub async fn definition(&self, params: Value) -> Result<(Value, LspTiming), LspError> {
        self.request_timed(LspMethod::Definition, params).await
    }
}

/// Reader task: frame messages, route responses by id, forward notifications.
async fn reader_loop<R>(
    reader: R,
    pending: PendingMap,
    notif_tx: mpsc::UnboundedSender<(String, Value)>,
) where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut br = BufReader::new(reader);
    loop {
        // ── headers ──
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            match br.read_line(&mut line).await {
                Ok(0) => return, // EOF
                Ok(_) => {}
                Err(_) => return,
            }
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                break; // end of headers
            }
            if let Some(rest) = trimmed.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = rest.trim().parse::<usize>().ok();
            }
        }

        let len = match content_length {
            Some(l) => l,
            None => continue, // malformed header block; resync on next message
        };

        // ── body ──
        let mut body = vec![0u8; len];
        if br.read_exact(&mut body).await.is_err() {
            return;
        }
        let value: Value = match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // ── route ──
        if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
            if let Some(tx) = pending.lock().unwrap().remove(&id) {
                let _ = tx.send(value);
            }
        } else if let Some(method) = value.get("method").and_then(|v| v.as_str()) {
            let params = value.get("params").cloned().unwrap_or(Value::Null);
            if notif_tx.send((method.to_string(), params)).is_err() {
                return; // consumer gone
            }
        }
    }
}

/// Writer task: write framed messages from the dispatch channel.
async fn writer_loop<W>(mut writer: W, mut rx: mpsc::UnboundedReceiver<Vec<u8>>)
where
    W: AsyncWrite + Unpin + Send + 'static,
{
    while let Some(frame) = rx.recv().await {
        if writer.write_all(&frame).await.is_err() {
            return;
        }
        if writer.flush().await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

    /// Read one framed LSP message from `reader` (test helper / fake server).
    async fn read_frame<R: AsyncRead + Unpin>(br: &mut BufReader<R>) -> Option<Value> {
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            if br.read_line(&mut line).await.ok()? == 0 {
                return None;
            }
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                break;
            }
            if let Some(rest) = trimmed.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = rest.trim().parse::<usize>().ok();
            }
        }
        let len = content_length?;
        let mut body = vec![0u8; len];
        br.read_exact(&mut body).await.ok()?;
        serde_json::from_slice(&body).ok()
    }

    fn frame(value: &Value) -> Vec<u8> {
        LspClient::frame(value)
    }

    #[tokio::test]
    async fn request_response_correlation_and_timing() {
        let (client_side, server_side) = tokio::io::duplex(8192);
        let (c_read, c_write) = tokio::io::split(client_side);
        let (s_read, mut s_write) = tokio::io::split(server_side);

        // Fake server: read one request, echo a result tagged with its id.
        tokio::spawn(async move {
            let mut br = BufReader::new(s_read);
            if let Some(req) = read_frame(&mut br).await {
                let id = req.get("id").cloned().unwrap_or(Value::Null);
                let resp = json!({"jsonrpc":"2.0","id": id, "result": {"ok": true}});
                let _ = s_write.write_all(&frame(&resp)).await;
                let _ = s_write.flush().await;
            }
        });

        let client = LspClient::new(c_read, c_write);
        let (result, timing) =
            client.completion(json!({"textDocument": {"uri": "file:///a.rs"}})).await.unwrap();

        assert_eq!(result, json!({"ok": true}));
        assert_eq!(timing.method, LspMethod::Completion);
        assert!(timing.total_round_trip_ms() >= 0.0);
    }

    #[tokio::test]
    async fn notifications_are_delivered() {
        let (client_side, server_side) = tokio::io::duplex(8192);
        let (c_read, c_write) = tokio::io::split(client_side);
        let (_s_read, mut s_write) = tokio::io::split(server_side);

        let client = LspClient::new(c_read, c_write);
        let mut notifs = client.take_notifications().unwrap();

        let note = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {"uri": "file:///a.rs", "diagnostics": []}
        });
        s_write.write_all(&frame(&note)).await.unwrap();
        s_write.flush().await.unwrap();

        let (method, params) = notifs.recv().await.unwrap();
        assert_eq!(method, "textDocument/publishDiagnostics");
        assert_eq!(params.get("uri").unwrap(), "file:///a.rs");
    }
}
