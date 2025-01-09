pub mod http;
pub mod stdio;
// pub mod ws;

pub use stdio::StdioTransport;

use crate::protocol::messages::JSONRPCMessage;
use crate::Result;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

#[async_trait]
pub trait Transport: Send + Sync + Clone {
    /// Send a message through the transport
    async fn send(&self, message: JSONRPCMessage) -> Result<()>;

    /// Receive a message from the transport
    async fn receive(&self) -> Result<JSONRPCMessage>;

    /// Get a stream of messages from the transport
    fn message_stream(&self) -> Option<Pin<Box<dyn Stream<Item = Result<JSONRPCMessage>> + Send>>>;
}
