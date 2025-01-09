use assistant::client::AssistantClient;
use assistant::error::Result;
use assistant::models::Model;
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
async fn test_assistant_client() {
    let mock_model = MockModel {
        response: "Hello, I am Claude!".to_string(),
    };

    let client = AssistantClient::new(Arc::new(mock_model));
    let result = client.send_message("Hello").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello, I am Claude!");
}

#[tokio::test]
async fn test_assistant_client_initialization() {
    let mock_model = MockModel {
        response: "Test response".to_string(),
    };

    let client = AssistantClient::new(Arc::new(mock_model));
    let result = client.initialize().await;

    assert!(result.is_ok());
}
