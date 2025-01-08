use crate::protocol::messages::*;
use crate::server::{CreateMessageParams, MessageHandler, ModelContextServer};
use crate::{Error, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

const NOTIFICATION_CHANNEL_SIZE: usize = 1024;

/// A high-performance implementation of the MCP server
pub struct MCPServer {
    info: Implementation,
    capabilities: ServerCapabilities,
    initialized: Arc<RwLock<bool>>,
    clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
    notification_tx: broadcast::Sender<JSONRPCNotification>,
}

struct ClientInfo {
    info: Implementation,
    capabilities: ClientCapabilities,
}

impl MCPServer {
    pub fn builder() -> MCPServerBuilder {
        MCPServerBuilder::default()
    }

    /// Broadcast a notification to all connected clients
    pub async fn broadcast_notification(&self, notification: JSONRPCNotification) -> Result<()> {
        // Using tokio's broadcast channel for efficient notification delivery
        let _ = self.notification_tx.send(notification);
        Ok(())
    }

    /// Create a new message handler for a client connection
    pub fn create_handler(&self) -> MessageHandler {
        MessageHandler::new(self.clone())
    }

    async fn ensure_initialized(&self) -> Result<()> {
        if !*self.initialized.read().await {
            Err(Error::Protocol("Server not initialized".into()))
        } else {
            Ok(())
        }
    }
}

impl Clone for MCPServer {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
            capabilities: self.capabilities.clone(),
            initialized: self.initialized.clone(),
            clients: self.clients.clone(),
            notification_tx: self.notification_tx.clone(),
        }
    }
}

#[async_trait]
impl ModelContextServer for MCPServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut clients = self.clients.write().await;

        // Store client info using a unique identifier (could be connection ID)
        let client_id = uuid::Uuid::new_v4().to_string();
        clients.insert(
            client_id,
            ClientInfo {
                info: params.client_info,
                capabilities: params.capabilities,
            },
        );

        // Mark as initialized
        *self.initialized.write().await = true;

        Ok(InitializeResult {
            protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
            capabilities: self.capabilities.clone(),
            server_info: self.info.clone(),
            instructions: None,
        })
    }

    async fn handle_ping(&self) -> Result<()> {
        self.ensure_initialized().await
    }

    async fn create_message(&self, params: CreateMessageParams) -> Result<CreateMessageResult> {
        self.ensure_initialized().await?;

        // Implementation would depend on your specific LLM integration
        Err(Error::Protocol("Not implemented".into()))
    }

    async fn list_roots(&self) -> Result<ListRootsResult> {
        self.ensure_initialized().await?;

        Ok(ListRootsResult {
            roots: Vec::new(), // Implement root listing based on your needs
        })
    }

    async fn handle_roots_changed(&self) -> Result<()> {
        self.ensure_initialized().await?;

        self.broadcast_notification(JSONRPCNotification {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: "notifications/roots/list_changed".to_string(),
            params: None,
        })
        .await
    }

    async fn handle_cancelled(&self, request_id: RequestId, reason: Option<String>) -> Result<()> {
        // Implementation for handling cancellation
        Ok(())
    }

    async fn handle_progress(
        &self,
        token: ProgressToken,
        progress: f64,
        total: Option<f64>,
    ) -> Result<()> {
        // Implementation for handling progress updates
        Ok(())
    }
}

#[derive(Default)]
pub struct MCPServerBuilder {
    info: Option<Implementation>,
    capabilities: Option<ServerCapabilities>,
}

impl MCPServerBuilder {
    pub fn info(mut self, info: Implementation) -> Self {
        self.info = Some(info);
        self
    }

    pub fn capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    pub fn build(self) -> Result<MCPServer> {
        let info = self
            .info
            .ok_or_else(|| Error::Internal("Server info is required".into()))?;

        let capabilities = self.capabilities.unwrap_or_default();
        let (notification_tx, _) = broadcast::channel(NOTIFICATION_CHANNEL_SIZE);

        Ok(MCPServer {
            info,
            capabilities,
            initialized: Arc::new(RwLock::new(false)),
            clients: Arc::new(RwLock::new(HashMap::new())),
            notification_tx,
        })
    }
}
