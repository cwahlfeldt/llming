use assistant::error::Result;
use assistant::models::Model;
use assistant::protocol::{handler::McpHandler, ProtocolHandler};
use async_trait::async_trait;
use std::sync::Arc;

struct MockModel {
    response: String,
}

#[async_trait]
impl Model for MockModel {
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    async fn send_message(&self, _message: &str) -> Result<String> {
        Ok(self.response.clone())
    }

    fn id(&self) -> &str {
        "mock"
    }

    fn supports_streaming(&self) -> bool {
        false
    }
}

#[tokio::test]
async fn test_mcp_handler_initialization() {
    let mock_model = MockModel {
        response: "Test response".to_string(),
    };

    let handler = McpHandler::new(Arc::new(mock_model));
    let result = handler.initialize().await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mcp_handler_shutdown() {
    let mock_model = MockModel {
        response: "Test response".to_string(),
    };

    let handler = McpHandler::new(Arc::new(mock_model));
    let result = handler.shutdown().await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mcp_handler_message() {
    let mock_model = MockModel {
        response: "Test response".to_string(),
    };

    let handler = McpHandler::new(Arc::new(mock_model));
    let message = b"test message";
    let result = handler.handle_message(message).await;

    assert!(result.is_ok());
}
