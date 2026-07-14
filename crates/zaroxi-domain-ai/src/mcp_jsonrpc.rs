//! JSON-RPC 2.0 types for MCP (Model Context Protocol) message exchange.
//!
//! These are pure domain types for serialization/deserialization of JSON-RPC
//! request, response, notification, and error messages. Uses `serde_json::Value`
//! for parameters and results, allowing schema-free transport.
//!
//! Implements the MCP base protocol per specification:
//! <https://spec.modelcontextprotocol.io/specification/2024-11-05/basic/>

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A JSON-RPC 2.0 request message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(id: JsonRpcId, method: &str, params: Option<Value>) -> Self {
        Self { jsonrpc: "2.0".into(), id, method: method.into(), params }
    }
}

/// A JSON-RPC 2.0 response message (success).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

/// A JSON-RPC 2.0 error response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub jsonrpc: String,
    pub id: Option<JsonRpcId>,
    pub error: JsonRpcErrorDetail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcErrorDetail {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// A JSON-RPC 2.0 notification (no id, no response expected).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    pub fn new(method: &str, params: Option<Value>) -> Self {
        Self { jsonrpc: "2.0".into(), method: method.into(), params }
    }
}

/// JSON-RPC request identifier — can be string or number.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    String(String),
    Number(i64),
}

/// Convenience: create a numeric id.
impl From<i64> for JsonRpcId {
    fn from(n: i64) -> Self {
        JsonRpcId::Number(n)
    }
}

impl From<&str> for JsonRpcId {
    fn from(s: &str) -> Self {
        JsonRpcId::String(s.into())
    }
}

impl std::fmt::Display for JsonRpcId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonRpcId::String(s) => write!(f, "{s}"),
            JsonRpcId::Number(n) => write!(f, "{n}"),
        }
    }
}

/// All possible incoming JSON-RPC messages.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Error(JsonRpcError),
    Notification(JsonRpcNotification),
}

/// Parse a JSON string into a `JsonRpcMessage`.
pub fn parse_json_rpc(line: &str) -> Result<JsonRpcMessage, String> {
    let value: Value = serde_json::from_str(line).map_err(|e| format!("JSON parse error: {e}"))?;

    // Detect message type by presence of fields
    if value.get("method").is_some() && value.get("id").is_some() {
        let req: JsonRpcRequest =
            serde_json::from_value(value).map_err(|e| format!("request parse: {e}"))?;
        Ok(JsonRpcMessage::Request(req))
    } else if value.get("method").is_some() {
        let notif: JsonRpcNotification =
            serde_json::from_value(value).map_err(|e| format!("notification parse: {e}"))?;
        Ok(JsonRpcMessage::Notification(notif))
    } else if value.get("error").is_some() {
        let err: JsonRpcError =
            serde_json::from_value(value).map_err(|e| format!("error parse: {e}"))?;
        Ok(JsonRpcMessage::Error(err))
    } else if value.get("result").is_some() || value.get("id").is_some() {
        let resp: JsonRpcResponse =
            serde_json::from_value(value).map_err(|e| format!("response parse: {e}"))?;
        Ok(JsonRpcMessage::Response(resp))
    } else {
        Err(format!("unrecognised JSON-RPC message: {line}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
        let msg = parse_json_rpc(json).unwrap();
        match msg {
            JsonRpcMessage::Request(req) => {
                assert_eq!(req.method, "tools/list");
                assert_eq!(req.id, JsonRpcId::Number(1));
            }
            _ => panic!("expected request"),
        }
    }

    #[test]
    fn parse_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let msg = parse_json_rpc(json).unwrap();
        match msg {
            JsonRpcMessage::Response(resp) => {
                assert_eq!(resp.id, JsonRpcId::Number(1));
                assert!(resp.result.is_some());
            }
            _ => panic!("expected response"),
        }
    }

    #[test]
    fn parse_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let msg = parse_json_rpc(json).unwrap();
        assert!(matches!(msg, JsonRpcMessage::Notification(_)));
    }

    #[test]
    fn parse_error() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let msg = parse_json_rpc(json).unwrap();
        match msg {
            JsonRpcMessage::Error(err) => {
                assert_eq!(err.error.code, -32601);
                assert_eq!(err.id, Some(JsonRpcId::Number(1)));
            }
            _ => panic!("expected error"),
        }
    }

    #[test]
    fn roundtrip_request() {
        let req = JsonRpcRequest::new(1.into(), "initialize", None);
        let json = serde_json::to_string(&req).unwrap();
        let back: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.method, "initialize");
    }

    #[test]
    fn json_rpc_id_display() {
        assert_eq!(JsonRpcId::Number(42).to_string(), "42");
        assert_eq!(JsonRpcId::String("abc".into()).to_string(), "abc");
    }
}
