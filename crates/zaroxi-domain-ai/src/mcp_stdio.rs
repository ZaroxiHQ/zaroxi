//! MCP stdio transport — concrete implementation of `McpTransportConnection`
//! that communicates with an MCP server over a child process's stdin/stdout.
//!
//! Each JSON-RPC message is a single line terminated by newline.
//! A background reader thread collects incoming messages; outgoing messages
//! are written directly to stdin.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;

use crate::mcp::McpTransport;
use crate::mcp_jsonrpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, parse_json_rpc,
};
use crate::mcp_transport::McpTransportConnection;

pub struct StdioTransport {
    child: Option<Child>,
    stdin: Option<std::process::ChildStdin>,
    reader_thread: Option<std::thread::JoinHandle<()>>,
    rx_pending: mpsc::Receiver<(Option<JsonRpcId>, JsonRpcMessage)>,
    alive: bool,
    next_id: i64,
}

impl StdioTransport {
    pub fn spawn(transport: &McpTransport) -> Result<Self, String> {
        let (command, args, env) = match transport {
            McpTransport::Stdio { command, args, env } => {
                (command.clone(), args.clone(), env.clone())
            }
            McpTransport::Sse { .. } => {
                return Err("StdioTransport cannot handle SSE config".into());
            }
        };

        let mut cmd = Command::new(&command);
        cmd.args(&args);
        for (key, val) in &env {
            cmd.env(key, val);
        }
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());

        let mut child = cmd.spawn().map_err(|e| format!("failed to spawn {command}: {e}"))?;

        let stdin = child.stdin.take().ok_or_else(|| "child has no stdin".to_string())?;
        let stdout = child.stdout.take().ok_or_else(|| "child has no stdout".to_string())?;

        let (tx_out, rx_pending) = mpsc::channel::<(Option<JsonRpcId>, JsonRpcMessage)>();

        let reader_thread = std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => break,
                };
                if line.trim().is_empty() {
                    continue;
                }
                let msg = match parse_json_rpc(&line) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let id = msg_id(&msg);
                if tx_out.send((id, msg)).is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            child: Some(child),
            stdin: Some(stdin),
            reader_thread: Some(reader_thread),
            rx_pending,
            alive: true,
            next_id: 1,
        })
    }

    /// Send a JSON-RPC request and wait synchronously for the matching response.
    pub fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<JsonRpcResponse, String> {
        let id: JsonRpcId = self.next_id.into();
        self.next_id += 1;
        let req = JsonRpcRequest::new(id.clone(), method, params);
        let json = serde_json::to_string(&req).map_err(|e| format!("serialize: {e}"))?;
        self.send(&json)?;

        let timeout_ms = 30_000u128;
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms {
            match self.rx_pending.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok((msg_id, msg)) => {
                    if msg_id.as_ref() == Some(&id) {
                        match msg {
                            JsonRpcMessage::Response(resp) => return Ok(resp),
                            JsonRpcMessage::Error(err) => {
                                return Err(format!(
                                    "MCP error [{}]: {}",
                                    err.error.code, err.error.message
                                ));
                            }
                            _ => {}
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("reader thread disconnected".into());
                }
            }
        }
        Err(format!("request {method} timed out"))
    }

    /// Send a notification (no response expected).
    pub fn send_notification(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let notif = JsonRpcNotification::new(method, params);
        let json = serde_json::to_string(&notif).map_err(|e| format!("serialize: {e}"))?;
        self.send(&json)
    }

    /// Drain any pending messages.
    pub fn drain_pending(&self) {
        while self.rx_pending.try_recv().is_ok() {}
    }
}

impl McpTransportConnection for StdioTransport {
    fn send(&mut self, json: &str) -> Result<(), String> {
        if !self.alive {
            return Err("transport not alive".into());
        }
        let mut line = json.trim().to_string();
        line.push('\n');
        let stdin = self.stdin.as_mut().ok_or("stdin not available")?;
        stdin.write_all(line.as_bytes()).map_err(|e| format!("write failed: {e}"))?;
        stdin.flush().map_err(|e| format!("flush failed: {e}"))?;
        Ok(())
    }

    fn receive(&mut self) -> Result<JsonRpcMessage, String> {
        match self.rx_pending.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok((_, msg)) => Ok(msg),
            Err(mpsc::RecvTimeoutError::Timeout) => Err("receive timed out".into()),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err("reader disconnected".into()),
        }
    }

    fn is_alive(&self) -> bool {
        self.alive
    }

    fn close(&mut self) -> Result<(), String> {
        self.alive = false;
        drop(self.stdin.take());
        if let Some(h) = self.reader_thread.take() {
            let _ = h.join();
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn msg_id(msg: &JsonRpcMessage) -> Option<JsonRpcId> {
    match msg {
        JsonRpcMessage::Request(req) => Some(req.id.clone()),
        JsonRpcMessage::Response(resp) => Some(resp.id.clone()),
        JsonRpcMessage::Error(err) => err.id.clone(),
        JsonRpcMessage::Notification(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdio_transport_errors_on_sse_config() {
        let config = McpTransport::Sse { url: "http://localhost".into(), headers: vec![] };
        assert!(StdioTransport::spawn(&config).is_err());
    }

    #[test]
    fn stdio_transport_with_echo_command() {
        let config = McpTransport::Stdio { command: "cat".into(), args: vec![], env: vec![] };
        let mut t = StdioTransport::spawn(&config).unwrap();
        assert!(t.is_alive());

        t.send(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).unwrap();
        let msg = t.receive();
        assert!(msg.is_ok());

        t.close().unwrap();
        assert!(!t.is_alive());
    }
}
