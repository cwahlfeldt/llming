use std::io::{self, BufRead, Write};
use std::future::Future;
use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{
    AsyncBufReadExt, BufReader, 
    AsyncWriteExt, stdout, stdin,
};
use mcp_schema::{JSONRPCRequest, JSONRPCResponse, JSONRPCNotification};

use crate::{Result, Error};
use super::{Transport, Message};

/// A transport that communicates over stdin/stdout using JSON-RPC messages.
pub struct StdioTransport {
    reader: BufReader<tokio::io::Stdin>,
    writer: tokio::io::Stdout,
}

impl StdioTransport {
    /// Create a new transport that reads from stdin and writes to stdout.
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(stdin()),
            writer: stdout(),
        }
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn receive(&mut self) -> Result<Message> {
        let mut line = String::new();
        self.reader.read_line(&mut line).await.map_err(|e| {
            Error::Internal(format!("failed to read line: {}", e))
        })?;

        // Parse as generic JSON first
        let value: Value = serde_json::from_str(&line).map_err(|e| {
            Error::InvalidRequest(format!("invalid JSON: {}", e))
        })?;

        // Try to determine message type from the JSON structure
        if let Some(method) = value.get("method") {
            if value.get("id").is_some() {
                // Has method and id = request
                let request: JSONRPCRequest<Value> = serde_json::from_value(value).map_err(|e| {
                    Error::InvalidRequest(format!("invalid request: {}", e))
                })?;
                Ok(Message::Request(request))
            } else {
                // Has method but no id = notification
                let notification: JSONRPCNotification<Value> = serde_json::from_value(value).map_err(|e| {
                    Error::InvalidRequest(format!("invalid notification: {}", e))
                })?;
                Ok(Message::Notification(notification))
            }
        } else {
            // No method = response
            let response: JSONRPCResponse<Value> = serde_json::from_value(value).map_err(|e| {
                Error::InvalidRequest(format!("invalid response: {}", e))
            })?;
            Ok(Message::Response(response))
        }
    }

    async fn send(&mut self, message: Message) -> Result<()> {
        // Convert message to JSON
        let value = match message {
            Message::Request(req) => serde_json::to_value(req)?,
            Message::Notification(notif) => serde_json::to_value(notif)?,
            Message::Response(resp) => serde_json::to_value(resp)?,
        };

        // Write JSON followed by newline
        let mut output = serde_json::to_string(&value)?;
        output.push('\n');
        
        self.writer.write_all(output.as_bytes()).await.map_err(|e| {
            Error::Internal(format!("failed to write: {}", e))
        })?;

        self.writer.flush().await.map_err(|e| {
            Error::Internal(format!("failed to flush: {}", e))
        })?;

        Ok(())
    }

    async fn run<F, Fut>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(Message) -> Fut + Send + Sync,
        Fut: Future<Output = Result<Option<Message>>> + Send,
    {
        loop {
            // Receive next message
            let message = self.receive().await?;

            // Process message
            if let Some(response) = handler(message).await? {
                // Send response if we got one
                self.send(response).await?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_schema::{JSONRPC_VERSION, RequestId};
    use serde_json::json;

    fn test_request() -> Message {
        Message::Request(JSONRPCRequest {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: RequestId::Number(1),
            method: "test".to_string(),
            params: json!({}),
        })
    }

    fn test_notification() -> Message {
        Message::Notification(JSONRPCNotification {
            json_rpc: JSONRPC_VERSION.to_string(),
            method: "test".to_string(),
            params: json!({}),
        })
    }

    fn test_response() -> Message {
        Message::Response(JSONRPCResponse {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: RequestId::Number(1),
            result: json!({}),
        })
    }

    // Note: We can't easily test actual stdin/stdout here
    // Instead we test message serialization/deserialization logic

    #[test]
    fn test_message_serialization() {
        // Test request
        let req = test_request();
        let value = match req {
            Message::Request(req) => serde_json::to_value(req).unwrap(),
            _ => panic!("expected request"),
        };
        assert!(value.get("method").is_some());
        assert!(value.get("id").is_some());

        // Test notification
        let notif = test_notification();
        let value = match notif {
            Message::Notification(notif) => serde_json::to_value(notif).unwrap(),
            _ => panic!("expected notification"),
        };
        assert!(value.get("method").is_some());
        assert!(value.get("id").is_none());

        // Test response
        let resp = test_response();
        let value = match resp {
            Message::Response(resp) => serde_json::to_value(resp).unwrap(),
            _ => panic!("expected response"),
        };
        assert!(value.get("result").is_some());
        assert!(value.get("id").is_some());
    }
}
