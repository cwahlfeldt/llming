use crate::{protocol::messages::*, RequestId};
use crate::{Error, ModelPreferences, ProgressToken, Result, JSONRPC_VERSION};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[async_trait]
pub trait ModelContextServer: Send + Sync {
    /// Handle initialization request from a client
    async fn initialize(&self, params: InitializeRequest) -> Result<InitializeResult>;

    /// Handle a ping request
    async fn handle_ping(&self) -> Result<()>;

    /// Handle sampling request from a client
    async fn create_message(&self, params: CreateMessageRequest) -> Result<CreateMessageResult>;

    /// Handle roots listing request
    async fn list_roots(&self) -> Result<ListRootsResult>;

    /// Notify the server when roots have changed
    async fn handle_roots_changed(&self) -> Result<()>;

    /// Handle a cancelled notification
    async fn handle_cancelled(&self, request_id: RequestId, reason: Option<String>) -> Result<()>;

    /// Handle a progress notification
    async fn handle_progress(
        &self,
        token: ProgressToken,
        progress: f64,
        total: Option<f64>,
    ) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageParams {
    pub messages: Vec<SamplingMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<IncludeContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    pub max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IncludeContext {
    None,
    ThisServer,
    AllServers,
}

// High-performance message handler using a channel-based approach
pub struct MessageHandler {
    inner: Arc<dyn ModelContextServer>,
}

impl MessageHandler {
    pub fn new<S: ModelContextServer + 'static>(server: S) -> Self {
        Self {
            inner: Arc::new(server),
        }
    }

    pub async fn handle_message(&self, message: JSONRPCMessage) -> Result<Option<JSONRPCMessage>> {
        match message {
            JSONRPCMessage::Request(req) => self
                .handle_request(req)
                .await
                .map(|r| Some(JSONRPCMessage::Response(r))),
            JSONRPCMessage::Notification(notif) => {
                self.handle_notification(notif).await.map(|_| None)
            }
            _ => Ok(None),
        }
    }

    async fn handle_request(&self, request: JSONRPCRequest) -> Result<JSONRPCResponse> {
        let result = match request.method.as_str() {
            "initialize" => {
                if let Some(RequestParams::Initialize(params)) = request.params {
                    ResponseResult::Initialize(self.inner.initialize(params).await?)
                } else {
                    return Err(Error::InvalidParams(
                        "Invalid or missing parameters for initialize".into(),
                    ));
                }
            }
            "ping" => {
                if let Some(RequestParams::Ping(_)) = request.params {
                    self.inner.handle_ping().await?;
                    ResponseResult::Empty(())
                } else {
                    return Err(Error::InvalidParams(
                        "Invalid or missing parameters for ping".into(),
                    ));
                }
            }
            "sampling/createMessage" => {
                if let Some(RequestParams::CreateMessage(params)) = request.params {
                    ResponseResult::CreateMessage(self.inner.create_message(params).await?)
                } else {
                    return Err(Error::InvalidParams(
                        "Invalid or missing parameters for createMessage".into(),
                    ));
                }
            }
            "roots/list" => {
                if let Some(RequestParams::ListRoots(_)) = request.params {
                    ResponseResult::ListRoots(self.inner.list_roots().await?)
                } else {
                    return Err(Error::InvalidParams(
                        "Invalid or missing parameters for list_roots".into(),
                    ));
                }
            }
            _ => return Err(Error::MethodNotFound(request.method)),
        };

        Ok(JSONRPCResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: request.id,
            result,
        })
    }

    async fn handle_notification(&self, notification: JSONRPCNotification) -> Result<()> {
        match notification.params {
            Some(NotificationParams::Cancelled(params)) => {
                self.inner
                    .handle_cancelled(params.request_id, params.reason)
                    .await
            }
            Some(NotificationParams::Progress(params)) => {
                self.inner
                    .handle_progress(params.progress_token, params.progress, params.total)
                    .await
            }
            Some(NotificationParams::RootsListChanged(_)) => {
                self.inner.handle_roots_changed().await
            }
            _ => Ok(()),
        }
    }
}
