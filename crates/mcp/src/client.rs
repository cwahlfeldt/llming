use crate::protocol::messages::*;
use crate::{Cursor, RequestId, Result};
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait ModelContextClient: Send + Sync {
    /// Initialize the client with the server
    async fn initialize(&self) -> Result<InitializeResult>;

    /// Send a notification that initialization is complete
    async fn initialized(&self) -> Result<()>;

    /// Send a ping request
    async fn ping(&self) -> Result<()>;

    /// List available resources
    async fn list_resources(&self, cursor: Option<Cursor>) -> Result<ListResourcesResult>;

    /// List resource templates
    async fn list_resource_templates(
        &self,
        cursor: Option<Cursor>,
    ) -> Result<ListResourceTemplatesResult>;

    /// Read a specific resource
    async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult>;

    /// Subscribe to resource updates
    async fn subscribe(&self, uri: &str) -> Result<()>;

    /// Unsubscribe from resource updates
    async fn unsubscribe(&self, uri: &str) -> Result<()>;

    /// List available prompts
    async fn list_prompts(&self, cursor: Option<Cursor>) -> Result<ListPromptsResult>;

    /// Get a specific prompt
    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, String>>,
    ) -> Result<GetPromptResult>;

    /// List available tools
    async fn list_tools(&self, cursor: Option<Cursor>) -> Result<ListToolsResult>;

    /// Call a specific tool
    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<CallToolResult>;

    /// Set logging level
    async fn set_logging_level(&self, level: LoggingLevel) -> Result<()>;

    /// Send a completion request
    async fn complete(&self, request: CompleteRequest) -> Result<CompleteResult>;

    /// Cancel a request
    async fn cancel(&self, request_id: RequestId, reason: Option<String>) -> Result<()>;
}
