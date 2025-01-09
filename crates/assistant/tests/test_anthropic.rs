use assistant::models::anthropic::{AnthropicClient, ChatMessage, Message};
use assistant::models::Model;
use http_body_util::Full;
use hyper::{body::Bytes, Response};
use hyperax::client::Client;
use std::sync::Arc;

#[tokio::test]
async fn test_anthropic_client_initialization() {
    client.mock(Response::new(Bytes::from("{}")));

    let client = AnthropicClient::new("test-key".to_string(), "claude-3".to_string());
    let result = client.initialize().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_anthropic_client_send_message() {
    let response_json = r#"{
        "content": [{"type": "text", "text": "Hello human!"}],
        "model": "claude-3",
        "role": "assistant"
    }"#;

    client.mock(Response::new(Bytes::from(response_json)));

    let client = AnthropicClient::new("test-key".to_string(), "claude-3".to_string());
    let result = client.send_message("Hello").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello human!");
}

#[tokio::test]
async fn test_anthropic_client_error_handling() {
    let mut response = Response::new(Bytes::from("{}"));
    *response.status_mut() = hyper::StatusCode::UNAUTHORIZED;
    client.mock(response);

    let client = AnthropicClient::new("invalid-key".to_string(), "claude-3".to_string());
    let result = client.send_message("Test message").await;

    assert!(result.is_err());
}
