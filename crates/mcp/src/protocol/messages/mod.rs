mod completion;
mod control;
mod initialize;
mod logging;
mod prompts;
mod resources;
mod roots;
mod sampling;
mod tools;

pub use completion::*;
pub use control::*;
pub use initialize::*;
pub use logging::*;
pub use prompts::*;
pub use resources::*;
pub use roots::*;
pub use sampling::*;
pub use tools::*;

use crate::protocol::common::*;
use crate::protocol::constants::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JSONRPCMessage {
    Request(JSONRPCRequest),
    Notification(JSONRPCNotification),
    Response(JSONRPCResponse),
    Error(JSONRPCError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<RequestParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    pub result: ResponseResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCError {
    pub jsonrpc: String,
    pub id: RequestId,
    pub error: JSONRPCErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCErrorDetail {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum RequestParams {
    #[serde(rename = "initialize")]
    Initialize(InitializeRequest),
    #[serde(rename = "ping")]
    Ping(PingRequest),
    #[serde(rename = "resources/list")]
    ListResources(ListResourcesRequest),
    #[serde(rename = "resources/templates/list")]
    ListResourceTemplates(ListResourceTemplatesRequest),
    #[serde(rename = "resources/read")]
    ReadResource(ReadResourceRequest),
    #[serde(rename = "resources/subscribe")]
    Subscribe(SubscribeRequest),
    #[serde(rename = "resources/unsubscribe")]
    Unsubscribe(UnsubscribeRequest),
    #[serde(rename = "prompts/list")]
    ListPrompts(ListPromptsRequest),
    #[serde(rename = "prompts/get")]
    GetPrompt(GetPromptRequest),
    #[serde(rename = "tools/list")]
    ListTools(ListToolsRequest),
    #[serde(rename = "tools/call")]
    CallTool(CallToolRequest),
    #[serde(rename = "logging/setLevel")]
    SetLevel(SetLevelRequest),
    #[serde(rename = "completion/complete")]
    Complete(CompleteRequest),
    #[serde(rename = "roots/list")]
    ListRoots(ListRootsRequest),
    #[serde(rename = "sampling/createMessage")]
    CreateMessage(CreateMessageRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum NotificationParams {
    #[serde(rename = "notifications/cancelled")]
    Cancelled(CancelledNotification),
    #[serde(rename = "notifications/progress")]
    Progress(ProgressNotification),
    #[serde(rename = "notifications/initialized")]
    Initialized(InitializedNotification),
    #[serde(rename = "notifications/message")]
    LoggingMessage(LoggingMessageNotification),
    #[serde(rename = "notifications/resources/updated")]
    ResourceUpdated(ResourceUpdatedNotification),
    #[serde(rename = "notifications/resources/list_changed")]
    ResourceListChanged(ResourceListChangedNotification),
    #[serde(rename = "notifications/tools/list_changed")]
    ToolListChanged(ToolListChangedNotification),
    #[serde(rename = "notifications/prompts/list_changed")]
    PromptListChanged(PromptListChangedNotification),
    #[serde(rename = "notifications/roots/list_changed")]
    RootsListChanged(RootsListChangedNotification),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseResult {
    Initialize(InitializeResult),
    ListResources(ListResourcesResult),
    ListResourceTemplates(ListResourceTemplatesResult),
    ReadResource(ReadResourceResult),
    ListPrompts(ListPromptsResult),
    GetPrompt(GetPromptResult),
    ListTools(ListToolsResult),
    CallTool(CallToolResult),
    Complete(CompleteResult),
    ListRoots(ListRootsResult),
    CreateMessage(CreateMessageResult),
    Empty(()),
}

// Helper methods for creating messages
impl JSONRPCMessage {
    fn next_id() -> RequestId {
        REQUEST_ID_COUNTER.fetch_add(1, Ordering::SeqCst).into()
    }

    pub fn request(method: &str, params: Option<RequestParams>) -> Self {
        Self::Request(JSONRPCRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: Self::next_id(),
            method: method.to_string(),
            params,
        })
    }

    pub fn notification(method: &str, params: Option<NotificationParams>) -> Self {
        Self::Notification(JSONRPCNotification {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.to_string(),
            params,
        })
    }

    pub fn response(id: RequestId, result: ResponseResult) -> Self {
        Self::Response(JSONRPCResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result,
        })
    }

    pub fn error(
        id: RequestId,
        code: i32,
        message: String,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self::Error(JSONRPCError {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            error: JSONRPCErrorDetail {
                code,
                message,
                data,
            },
        })
    }

    pub fn id(&self) -> Option<&RequestId> {
        match self {
            Self::Request(req) => Some(&req.id),
            Self::Response(res) => Some(&res.id),
            Self::Error(err) => Some(&err.id),
            Self::Notification(_) => None,
        }
    }

    pub fn method(&self) -> Option<&str> {
        match self {
            Self::Request(req) => Some(&req.method),
            Self::Notification(notif) => Some(&notif.method),
            _ => None,
        }
    }

    pub fn is_request(&self) -> bool {
        matches!(self, Self::Request(_))
    }

    pub fn is_notification(&self) -> bool {
        matches!(self, Self::Notification(_))
    }

    pub fn is_response(&self) -> bool {
        matches!(self, Self::Response(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

// Extension trait for common message creation patterns
pub trait MessageFactory {
    fn initialize_request(
        capabilities: ClientCapabilities,
        client_info: Implementation,
    ) -> JSONRPCMessage {
        JSONRPCMessage::request(
            "initialize",
            Some(RequestParams::Initialize(InitializeRequest {
                protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
                capabilities,
                client_info,
            })),
        )
    }

    fn initialized_notification() -> JSONRPCMessage {
        JSONRPCMessage::notification(
            "notifications/initialized",
            Some(NotificationParams::Initialized(InitializedNotification {})),
        )
    }

    fn ping_request() -> JSONRPCMessage {
        JSONRPCMessage::request("ping", Some(RequestParams::Ping(PingRequest {})))
    }

    fn cancelled_notification(request_id: RequestId, reason: Option<String>) -> JSONRPCMessage {
        JSONRPCMessage::notification(
            "notifications/cancelled",
            Some(NotificationParams::Cancelled(CancelledNotification {
                request_id,
                reason,
            })),
        )
    }
}

impl MessageFactory for JSONRPCMessage {}
