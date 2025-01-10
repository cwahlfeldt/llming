pub mod stdio;

use std::future::Future;
use serde_json::Value;
use mcp_schema::{JSONRPCRequest, JSONRPCResponse, JSONRPCNotification};
use async_trait::async_trait;
use crate::Result;

/// A trait for transports that can be used to communicate with MCP clients.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Receive the next message from the transport.
    async fn receive(&mut self) -> Result<Message>;

    /// Send a message over the transport.
    async fn send(&mut self, message: Message) -> Result<()>;

    /// Run the transport with the provided handler function.
    ///
    /// This will typically run in a loop until an error occurs or the transport is closed.
    async fn run<F, Fut>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(Message) -> Fut + Send + Sync,
        Fut: Future<Output = Result<Option<Message>>> + Send;
}

/// Represents a message that can be sent or received over a transport.
#[derive(Debug, Clone)]
pub enum Message {
    /// A request from the client that expects a response.
    Request(JSONRPCRequest<Value>),
    /// A notification from the client that doesn't expect a response.
    Notification(JSONRPCNotification<Value>),
    /// A response from the server.
    Response(JSONRPCResponse<Value>),
}
