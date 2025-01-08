pub mod client;
pub mod error;
pub mod protocol;
pub mod server;
pub mod transport;

pub use client::ModelContextClient;
pub use error::{Error, Result};
pub use server::ModelContextServer;

use async_trait::async_trait;
use protocol::{common::*, constants::*, messages::*};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use transport::stdio;

/// A high-performance MCP client implementation
pub struct MCPClient<T: transport::Transport> {
    transport: Arc<T>,
    client_info: Implementation,
    capabilities: ClientCapabilities,
    initialized: Arc<RwLock<bool>>,
    server_info: Arc<RwLock<Option<Implementation>>>,
    server_capabilities: Arc<RwLock<Option<ServerCapabilities>>>,
}

impl<T: transport::Transport> MCPClient<T> {
    pub fn builder() -> MCPClientBuilder<T> {
        MCPClientBuilder::default()
    }

    /// Get the server info if initialized
    pub async fn server_info(&self) -> Option<Implementation> {
        self.server_info.read().await.clone()
    }

    /// Get the server capabilities if initialized
    pub async fn server_capabilities(&self) -> Option<ServerCapabilities> {
        self.server_capabilities.read().await.clone()
    }

    async fn ensure_initialized(&self) -> Result<()> {
        if !*self.initialized.read().await {
            Err(Error::Protocol("Client not initialized".into()))
        } else {
            Ok(())
        }
    }

    async fn send_request<P: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Option<P>,
    ) -> Result<R> {
        let message = JSONRPCMessage::request(
            method,
            params
                .map(|p| serde_json::to_value(p).unwrap())
                .map(|v| match method {
                    "initialize" => RequestParams::Initialize(serde_json::from_value(v).unwrap()),
                    "ping" => RequestParams::Ping(PingRequest {}),
                    "resources/list" => {
                        RequestParams::ListResources(serde_json::from_value(v).unwrap())
                    }
                    "resources/templates/list" => {
                        RequestParams::ListResourceTemplates(serde_json::from_value(v).unwrap())
                    }
                    "resources/read" => {
                        RequestParams::ReadResource(serde_json::from_value(v).unwrap())
                    }
                    "resources/subscribe" => {
                        RequestParams::Subscribe(serde_json::from_value(v).unwrap())
                    }
                    "resources/unsubscribe" => {
                        RequestParams::Unsubscribe(serde_json::from_value(v).unwrap())
                    }
                    "prompts/list" => {
                        RequestParams::ListPrompts(serde_json::from_value(v).unwrap())
                    }
                    "prompts/get" => RequestParams::GetPrompt(serde_json::from_value(v).unwrap()),
                    "tools/list" => RequestParams::ListTools(serde_json::from_value(v).unwrap()),
                    "tools/call" => RequestParams::CallTool(serde_json::from_value(v).unwrap()),
                    "logging/setLevel" => {
                        RequestParams::SetLevel(serde_json::from_value(v).unwrap())
                    }
                    "completion/complete" => {
                        RequestParams::Complete(serde_json::from_value(v).unwrap())
                    }
                    "roots/list" => RequestParams::ListRoots(serde_json::from_value(v).unwrap()),
                    "sampling/createMessage" => {
                        RequestParams::CreateMessage(serde_json::from_value(v).unwrap())
                    }
                    _ => panic!("Unknown method: {}", method),
                }),
        );

        self.transport.send(message).await?;

        match self.transport.receive().await? {
            JSONRPCMessage::Response(response) => match response.result {
                ResponseResult::Initialize(result) => serde_json::to_value(result)
                    .map_err(Error::from)
                    .and_then(|v| serde_json::from_value(v).map_err(Error::from)),
                ResponseResult::ListResources(result) => serde_json::to_value(result)
                    .map_err(Error::from)
                    .and_then(|v| serde_json::from_value(v).map_err(Error::from)),
                ResponseResult::ListResourceTemplates(result) => serde_json::to_value(result)
                    .map_err(Error::from)
                    .and_then(|v| serde_json::from_value(v).map_err(Error::from)),
                ResponseResult::ReadResource(result) => serde_json::to_value(result)
                    .map_err(Error::from)
                    .and_then(|v| serde_json::from_value(v).map_err(Error::from)),
                ResponseResult::ListPrompts(result) => serde_json::to_value(result)
                    .map_err(Error::from)
                    .and_then(|v| serde_json::from_value(v).map_err(Error::from)),
                ResponseResult::GetPrompt(result) => serde_json::to_value(result)
                    .map_err(Error::from)
                    .and_then(|v| serde_json::from_value(v).map_err(Error::from)),
                ResponseResult::ListTools(result) => serde_json::to_value(result)
                    .map_err(Error::from)
                    .and_then(|v| serde_json::from_value(v).map_err(Error::from)),
                ResponseResult::CallTool(result) => serde_json::to_value(result)
                    .map_err(Error::from)
                    .and_then(|v| serde_json::from_value(v).map_err(Error::from)),
                ResponseResult::Complete(result) => serde_json::to_value(result)
                    .map_err(Error::from)
                    .and_then(|v| serde_json::from_value(v).map_err(Error::from)),
                ResponseResult::ListRoots(result) => serde_json::to_value(result)
                    .map_err(Error::from)
                    .and_then(|v| serde_json::from_value(v).map_err(Error::from)),
                ResponseResult::CreateMessage(result) => serde_json::to_value(result)
                    .map_err(Error::from)
                    .and_then(|v| serde_json::from_value(v).map_err(Error::from)),
                ResponseResult::Empty(_) => serde_json::to_value(())
                    .map_err(Error::from)
                    .and_then(|v| serde_json::from_value(v).map_err(Error::from)),
            },
            JSONRPCMessage::Error(error) => Err(Error::Protocol(error.error.message)),
            _ => Err(Error::Protocol("Unexpected response type".into())),
        }
    }

    async fn send_notification<P: serde::Serialize>(
        &self,
        method: &str,
        params: Option<P>,
    ) -> Result<()> {
        let message = JSONRPCMessage::notification(
            method,
            params
                .map(|p| serde_json::to_value(p).unwrap())
                .map(|v| match method {
                    "notifications/cancelled" => {
                        NotificationParams::Cancelled(serde_json::from_value(v).unwrap())
                    }
                    "notifications/progress" => {
                        NotificationParams::Progress(serde_json::from_value(v).unwrap())
                    }
                    "notifications/initialized" => {
                        NotificationParams::Initialized(InitializedNotification {})
                    }
                    "notifications/message" => {
                        NotificationParams::LoggingMessage(serde_json::from_value(v).unwrap())
                    }
                    "notifications/resources/updated" => {
                        NotificationParams::ResourceUpdated(serde_json::from_value(v).unwrap())
                    }
                    "notifications/resources/list_changed" => {
                        NotificationParams::ResourceListChanged(ResourceListChangedNotification {})
                    }
                    "notifications/tools/list_changed" => {
                        NotificationParams::ToolListChanged(ToolListChangedNotification {})
                    }
                    "notifications/prompts/list_changed" => {
                        NotificationParams::PromptListChanged(PromptListChangedNotification {})
                    }
                    "notifications/roots/list_changed" => {
                        NotificationParams::RootsListChanged(RootsListChangedNotification {})
                    }
                    _ => panic!("Unknown notification method: {}", method),
                }),
        );

        self.transport.send(message).await
    }
}

#[async_trait]
impl<T: transport::Transport> ModelContextClient for MCPClient<T> {
    async fn initialize(&self) -> Result<InitializeResult> {
        let params = InitializeRequest {
            protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
            capabilities: self.capabilities.clone(),
            client_info: self.client_info.clone(),
        };

        let result: InitializeResult = self.send_request("initialize", Some(params)).await?;

        {
            let mut initialized = self.initialized.write().await;
            let mut server_info = self.server_info.write().await;
            let mut server_capabilities = self.server_capabilities.write().await;

            *initialized = true;
            *server_info = Some(result.server_info.clone());
            *server_capabilities = Some(result.capabilities.clone());
        }

        Ok(result)
    }

    async fn initialized(&self) -> Result<()> {
        self.send_notification::<InitializedNotification>(
            "notifications/initialized",
            Some(InitializedNotification {}),
        )
        .await
    }

    async fn ping(&self) -> Result<()> {
        self.ensure_initialized().await?;
        self.send_request::<PingRequest, ()>("ping", Some(PingRequest {}))
            .await
    }

    async fn list_resources(&self, cursor: Option<Cursor>) -> Result<ListResourcesResult> {
        self.ensure_initialized().await?;
        self.send_request("resources/list", Some(ListResourcesRequest { cursor }))
            .await
    }

    async fn list_resource_templates(
        &self,
        cursor: Option<Cursor>,
    ) -> Result<ListResourceTemplatesResult> {
        self.ensure_initialized().await?;
        self.send_request(
            "resources/templates/list",
            Some(ListResourceTemplatesRequest { cursor }),
        )
        .await
    }

    async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult> {
        self.ensure_initialized().await?;
        self.send_request(
            "resources/read",
            Some(ReadResourceRequest {
                uri: uri.to_string(),
            }),
        )
        .await
    }

    async fn subscribe(&self, uri: &str) -> Result<()> {
        self.ensure_initialized().await?;
        self.send_request(
            "resources/subscribe",
            Some(SubscribeRequest {
                uri: uri.to_string(),
            }),
        )
        .await
    }

    async fn unsubscribe(&self, uri: &str) -> Result<()> {
        self.ensure_initialized().await?;
        self.send_request(
            "resources/unsubscribe",
            Some(UnsubscribeRequest {
                uri: uri.to_string(),
            }),
        )
        .await
    }

    async fn list_prompts(&self, cursor: Option<Cursor>) -> Result<ListPromptsResult> {
        self.ensure_initialized().await?;
        self.send_request("prompts/list", Some(ListPromptsRequest { cursor }))
            .await
    }

    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, String>>,
    ) -> Result<GetPromptResult> {
        self.ensure_initialized().await?;
        self.send_request(
            "prompts/get",
            Some(GetPromptRequest {
                name: name.to_string(),
                arguments,
            }),
        )
        .await
    }

    async fn list_tools(&self, cursor: Option<Cursor>) -> Result<ListToolsResult> {
        self.ensure_initialized().await?;
        self.send_request("tools/list", Some(ListToolsRequest { cursor }))
            .await
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<CallToolResult> {
        self.ensure_initialized().await?;
        self.send_request(
            "tools/call",
            Some(CallToolRequest {
                name: name.to_string(),
                arguments,
            }),
        )
        .await
    }

    async fn set_logging_level(&self, level: LoggingLevel) -> Result<()> {
        self.ensure_initialized().await?;
        self.send_request("logging/setLevel", Some(SetLevelRequest { level }))
            .await
    }

    async fn complete(&self, request: CompleteRequest) -> Result<CompleteResult> {
        self.ensure_initialized().await?;
        self.send_request("completion/complete", Some(request))
            .await
    }

    async fn cancel(&self, request_id: RequestId, reason: Option<String>) -> Result<()> {
        self.send_notification(
            "notifications/cancelled",
            Some(CancelledNotification { request_id, reason }),
        )
        .await
    }
}

#[derive(Default)]
pub struct MCPClientBuilder<T: transport::Transport> {
    transport: Option<T>,
    client_info: Option<Implementation>,
    capabilities: Option<ClientCapabilities>,
}

impl<T: transport::Transport> MCPClientBuilder<T> {
    pub fn default() -> Self {
        Self {
            transport: None,
            client_info: None,
            capabilities: None,
        }
    }

    pub fn transport(mut self, transport: T) -> Self {
        self.transport = Some(transport);
        self
    }

    pub fn client_info(mut self, info: Implementation) -> Self {
        self.client_info = Some(info);
        self
    }

    pub fn capabilities(mut self, capabilities: ClientCapabilities) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    pub fn build(self) -> Result<MCPClient<T>> {
        let transport = self
            .transport
            .ok_or_else(|| Error::Internal("Transport is required".into()))?;

        let client_info = self
            .client_info
            .ok_or_else(|| Error::Internal("Client info is required".into()))?;

        let capabilities = self.capabilities.unwrap_or_default();

        Ok(MCPClient {
            transport: Arc::new(transport),
            client_info,
            capabilities,
            initialized: Arc::new(RwLock::new(false)),
            server_info: Arc::new(RwLock::new(None)),
            server_capabilities: Arc::new(RwLock::new(None)),
        })
    }
}
