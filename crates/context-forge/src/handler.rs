//! Request handler traits and implementations.

use async_trait::async_trait;
use mcp_schema::ServerResult;
use serde_json::Value;

use crate::error::Result;

/// A trait for handling MCP requests.
/// 
/// This trait is implemented by types that can handle specific MCP methods.
/// Each handler is responsible for processing the request parameters and returning
/// an appropriate result or error.
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Handle a request with the given parameters.
    ///
    /// # Arguments
    /// * `params` - The parameters for the request as a raw JSON value.
    ///
    /// # Returns
    /// A Result containing the server's response or an error.
    async fn handle(&self, params: Value) -> Result<ServerResult>;
}

/// A trait for handling MCP notifications.
/// 
/// This trait is implemented by types that can handle specific MCP notifications.
/// Unlike RequestHandler, notification handlers don't return results.
#[async_trait]
pub trait NotificationHandler: Send + Sync {
    /// Handle a notification with the given parameters.
    ///
    /// # Arguments
    /// * `params` - The parameters for the notification as a raw JSON value.
    async fn handle(&self, params: Value) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestHandler;

    #[async_trait]
    impl RequestHandler for TestHandler {
        async fn handle(&self, _params: Value) -> Result<ServerResult> {
            Ok(ServerResult::Empty(mcp_schema::EmptyResult {
                meta: None,
                extra: Default::default(),
            }))
        }
    }

    #[tokio::test]
    async fn test_request_handler() {
        let handler = TestHandler;
        let result = handler.handle(json!({})).await.unwrap();
        match result {
            ServerResult::Empty(_) => {}
            _ => panic!("Expected empty result"),
        }
    }
}
