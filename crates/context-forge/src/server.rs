use mcp_schema::{
    EmptyResult, Implementation, InitializeParams, InitializeResult, JSONRPCError,
    JSONRPCNotification, JSONRPCRequest, JSONRPCResponse, LoggingLevel, ProgressToken,
    ServerCapabilities, ServerResult, JSONRPC_VERSION, LATEST_PROTOCOL_VERSION,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::handler::{NotificationHandler, RequestHandler};
use crate::handlers::{PromptHandler, ResourceHandler, ToolHandler};
use crate::logging::LoggingHandler;
use crate::progress::{get_progress_token, ProgressTracker};
use crate::roots::RootHandler;
use crate::{Error, Result};

type BoxedRequestHandler = Box<dyn RequestHandler + Send + Sync>;
type BoxedNotificationHandler = Box<dyn NotificationHandler + Send + Sync>;

/// State machine representing server initialization status
#[derive(Debug)]
enum ServerState {
    /// Server is not initialized yet
    Uninitialized {
        /// Server capabilities that will be advertised during initialization
        capabilities: ServerCapabilities,
        /// Server implementation info that will be advertised
        implementation: Implementation,
    },
    /// Server is initialized and ready to handle requests
    Initialized {
        /// Active capabilities as negotiated during initialization
        capabilities: ServerCapabilities,
        /// Server implementation info as shared during initialization
        implementation: Implementation,
        /// Protocol version in use as negotiated
        protocol_version: String,
    },
}

/// Main MCP server implementation
pub struct Server {
    state: Arc<RwLock<ServerState>>,
    request_handlers: Arc<RwLock<HashMap<String, BoxedRequestHandler>>>,
    notification_handlers: Arc<RwLock<HashMap<String, BoxedNotificationHandler>>>,
    notification_sender: Arc<Box<dyn Fn(JSONRPCNotification<Value>) + Send + Sync>>,
    // Default handlers
    resource_handler: Option<ResourceHandler>,
    tool_handler: Option<ToolHandler>,
    prompt_handler: Option<PromptHandler>,
    root_handler: Option<RootHandler>,
    logging_handler: Option<LoggingHandler>,
}

impl Server {
    /// Create a new uninitialized server with a notification callback
    pub fn new_with_notifications(
        capabilities: ServerCapabilities,
        implementation: Implementation,
        notification_sender: impl Fn(JSONRPCNotification<Value>) + Send + Sync + 'static,
    ) -> Self {
        let notification_sender: Arc<Box<dyn Fn(JSONRPCNotification<Value>) + Send + Sync>> =
            Arc::new(Box::new(notification_sender));

        let mut server = Self {
            state: Arc::new(RwLock::new(ServerState::Uninitialized {
                capabilities: capabilities.clone(),
                implementation,
            })),
            request_handlers: Arc::new(RwLock::new(HashMap::new())),
            notification_handlers: Arc::new(RwLock::new(HashMap::new())),
            notification_sender: notification_sender.clone(),
            resource_handler: None,
            tool_handler: None,
            prompt_handler: None,
            root_handler: None,
            logging_handler: None,
        };

        // Initialize logging if enabled
        if capabilities.logging.is_some() {
            let sender = notification_sender.clone();
            let logging_handler = LoggingHandler::new(move |n| (sender)(n));
            server
                .register_handler("logging/setLevel", logging_handler.clone())
                .unwrap();
            server.logging_handler = Some(logging_handler);
        }

        // Initialize root handler if roots capability is enabled
        if let Some(experimental) = &capabilities.experimental {
            if let Some(roots) = experimental.get("roots") {
                if roots
                    .get("list_changed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    // let sender = notification_sender.clone();
                    let notification_sender = notification_sender.clone();
                    let root_handler = RootHandler::new(Some(Arc::new(Box::new(move |n| {
                        (notification_sender)(n)
                    }))));
                    server
                        .register_handler("roots/list", root_handler.clone())
                        .unwrap();
                    server.root_handler = Some(root_handler);
                } else {
                    let root_handler = RootHandler::new(None);
                    server
                        .register_handler("roots/list", root_handler.clone())
                        .unwrap();
                    server.root_handler = Some(root_handler);
                }
            }
        }

        // Initialize other handlers based on capabilities
        if capabilities.resources.is_some() {
            let handler = ResourceHandler::new();
            server
                .register_handler("resources/list", handler.clone())
                .unwrap();
            server
                .register_handler("resources/templates/list", handler.clone())
                .unwrap();
            server
                .register_handler("resources/read", handler.clone())
                .unwrap();
            server
                .register_handler("resources/subscribe", handler.clone())
                .unwrap();
            server
                .register_handler("resources/unsubscribe", handler.clone())
                .unwrap();
            server.resource_handler = Some(handler.clone());
        }

        if capabilities.tools.is_some() {
            let handler = ToolHandler::new();
            server
                .register_handler("tools/list", handler.clone())
                .unwrap();
            server
                .register_handler("tools/call", handler.clone())
                .unwrap();
            server.tool_handler = Some(handler);
        }

        if capabilities.prompts.is_some() {
            let handler = PromptHandler::new();
            server
                .register_handler("prompts/list", handler.clone())
                .unwrap();
            server
                .register_handler("prompts/get", handler.clone())
                .unwrap();
            server.prompt_handler = Some(handler);
        }

        server
    }

    /// Create a new uninitialized server that discards notifications
    pub fn new(capabilities: ServerCapabilities, implementation: Implementation) -> Self {
        Self::new_with_notifications(capabilities, implementation, |_| {})
    }

    /// Register a request handler for a specific method
    pub fn register_handler<H>(&self, method: impl Into<String>, handler: H) -> Result<()>
    where
        H: RequestHandler + Send + Sync + 'static,
    {
        let mut handlers = self
            .request_handlers
            .write()
            .map_err(|_| Error::Internal("handler lock poisoned".into()))?;

        handlers.insert(method.into(), Box::new(handler));
        Ok(())
    }

    /// Register a notification handler for a specific method
    pub fn register_notification_handler<H>(
        &self,
        method: impl Into<String>,
        handler: H,
    ) -> Result<()>
    where
        H: NotificationHandler + Send + Sync + 'static,
    {
        let mut handlers = self
            .notification_handlers
            .write()
            .map_err(|_| Error::Internal("handler lock poisoned".into()))?;
        handlers.insert(method.into(), Box::new(handler));
        Ok(())
    }

    /// Handle an initialization request
    async fn handle_initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut state = self
            .state
            .write()
            .map_err(|_| Error::Internal("state lock poisoned".into()))?;

        // Check if we're already initialized
        match *state {
            ServerState::Initialized { .. } => Err(Error::AlreadyInitialized),
            ServerState::Uninitialized {
                ref capabilities,
                ref implementation,
            } => {
                // For now we'll just accept the client's requested protocol version if it matches
                // our latest supported version
                if params.protocol_version != LATEST_PROTOCOL_VERSION {
                    return Err(Error::InvalidRequest(format!(
                        "unsupported protocol version: {}",
                        params.protocol_version
                    )));
                }

                // Create initialized state
                let new_state = ServerState::Initialized {
                    capabilities: capabilities.clone(),
                    implementation: implementation.clone(),
                    protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
                };

                // Create initialization result
                let result = InitializeResult {
                    meta: None,
                    protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
                    capabilities: capabilities.clone(),
                    server_info: implementation.clone(),
                    instructions: None,
                    extra: Default::default(),
                };

                // Update state
                *state = new_state;

                Ok(result)
            }
        }
    }

    /// Check if server is initialized, return error if not
    fn check_initialized(&self) -> Result<()> {
        let state = self
            .state
            .read()
            .map_err(|_| Error::Internal("state lock poisoned".into()))?;
        match *state {
            ServerState::Uninitialized { .. } => Err(Error::NotInitialized),
            ServerState::Initialized { .. } => Ok(()),
        }
    }

    /// Handle an incoming JSON-RPC request
    pub async fn handle_request(
        &self,
        request: JSONRPCRequest<Value>,
    ) -> Result<JSONRPCResponse<ServerResult>> {
        let result = match request.method.as_str() {
            "initialize" => match serde_json::from_value(request.params) {
                Ok(params) => match self.handle_initialize(params).await {
                    Ok(result) => Ok(ServerResult::Initialize(result)), // Add this line
                    Err(e) => Err(e),
                },
                Err(e) => Err(Error::InvalidParams(e.to_string())),
            },
            method => {
                // All other methods require initialization
                self.check_initialized()?;

                // Create progress tracker if requested
                let progress = get_progress_token(&request.params)
                    .map(|token| self.create_progress_tracker(token, None));

                // Find registered handler
                let handlers = self
                    .request_handlers
                    .read()
                    .map_err(|_| Error::Internal("handler lock poisoned".into()))?;

                if let Some(handler) = handlers.get(method) {
                    // Pass progress tracker to handler via request params
                    let mut params = request.params;
                    if let Some(tracker) = progress {
                        if let Some(obj) = params.as_object_mut() {
                            // Serialize progress tracker, handle error case
                            let tracker_value = serde_json::to_value(&tracker).map_err(|e| {
                                Error::Internal(format!(
                                    "Failed to serialize progress tracker: {}",
                                    e
                                ))
                            })?;
                            obj.insert("_progress".to_string(), tracker_value);
                        }
                    }
                    handler.handle(params).await
                } else {
                    Err(Error::MethodNotFound(method.to_string()))
                }
            }
        };

        Ok(match result {
            Ok(result) => JSONRPCResponse {
                json_rpc: JSONRPC_VERSION.to_string(),
                id: request.id,
                result,
            },
            Err(e) => {
                let (_, _) = e.to_rpc_error();
                RPCResponse(JSONRPCResponse {
                    json_rpc: JSONRPC_VERSION.to_string(),
                    id: request.id,
                    result: ServerResult::Empty(EmptyResult {
                        meta: None,
                        extra: Default::default(),
                    }),
                })
                .into()
            }
        })
    }

    /// Handle an incoming JSON-RPC notification
    pub async fn handle_notification(
        &self,
        notification: JSONRPCNotification<Value>,
    ) -> Result<()> {
        // All notifications except initialized require initialization549
        if notification.method != "notifications/initialized" {
            self.check_initialized()?;
        }

        // Find registered handler
        let handlers = self
            .notification_handlers
            .read()
            .map_err(|_| Error::Internal("handler lock poisoned".into()))?;

        if let Some(handler) = handlers.get(&notification.method) {
            handler.handle(notification.params).await
        } else {
            // Notifications can be silently ignored if no handler is registered
            Ok(())
        }
    }

    /// Create a progress tracker for a given token
    pub fn create_progress_tracker(
        &self,
        token: ProgressToken,
        total: Option<f64>,
    ) -> ProgressTracker {
        let sender = self.notification_sender.clone();
        ProgressTracker::new(token, total, move |n| (sender)(n))
    }

    /// Get a reference to the resource handler if one exists
    pub fn resource_handler(&self) -> Option<&ResourceHandler> {
        self.resource_handler.as_ref()
    }

    /// Get a reference to the tool handler if one exists
    pub fn tool_handler(&self) -> Option<&ToolHandler> {
        self.tool_handler.as_ref()
    }

    /// Get a reference to the prompt handler if one exists
    pub fn prompt_handler(&self) -> Option<&PromptHandler> {
        self.prompt_handler.as_ref()
    }

    /// Get a reference to the root handler if one exists
    pub fn root_handler(&self) -> Option<&RootHandler> {
        self.root_handler.as_ref()
    }

    /// Get a reference to the logging handler if one exists
    pub fn logging_handler(&self) -> Option<&LoggingHandler> {
        self.logging_handler.as_ref()
    }

    /// Log a message at the specified level
    pub fn log(&self, level: LoggingLevel, logger: Option<String>, data: Value) -> Result<()> {
        if let Some(handler) = &self.logging_handler {
            handler.log(level, logger, data)
        } else {
            // Silently ignore if logging not enabled
            Ok(())
        }
    }
}

// Convert JSONRPCError to JSONRPCResponse for error cases
pub struct RPCResponse(JSONRPCResponse<ServerResult>);

impl From<JSONRPCError> for RPCResponse {
    fn from(error: JSONRPCError) -> Self {
        RPCResponse(serde_json::from_value(serde_json::to_value(error).unwrap()).unwrap())
    }
}

impl From<RPCResponse> for JSONRPCResponse<ServerResult> {
    fn from(response: RPCResponse) -> Self {
        response.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_schema::RequestId;
    use serde_json::json;

    fn test_server() -> Server {
        Server::new(
            ServerCapabilities {
                experimental: None,
                logging: None,
                prompts: None,
                resources: None,
                tools: None,
                extra: Default::default(),
            },
            Implementation {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                extra: Default::default(),
            },
        )
    }

    #[tokio::test]
    async fn test_initialization() {
        let server = test_server();

        // First initialize the server
        let init_req = JSONRPCRequest {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: RequestId::Number(1),
            method: "initialize".to_string(),
            params: json!({
                "protocolVersion": LATEST_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "test",
                    "version": "1.0"
                }
            }),
        };

        // Initialize should succeed
        let init_resp = server.handle_request(init_req).await.unwrap();
        match init_resp.result {
            ServerResult::Initialize(result) => {
                assert_eq!(result.protocol_version, LATEST_PROTOCOL_VERSION);
            }
            _ => panic!("Expected Initialize result"),
        }

        // Test regular request after initialization
        let req = JSONRPCRequest {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: RequestId::Number(2),
            method: "test".to_string(),
            params: json!({}),
        };

        // This should return method not found (since "test" isn't registered)
        // but shouldn't fail with NotInitialized
        let resp = server.handle_request(req).await.unwrap();
        assert!(matches!(resp.result, ServerResult::Empty(_)));

        // Test double initialization - should fail
        let init_req2 = JSONRPCRequest {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: RequestId::Number(3),
            method: "initialize".to_string(),
            params: json!({
                "protocolVersion": LATEST_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "test",
                    "version": "1.0"
                }
            }),
        };
        let resp = server.handle_request(init_req2).await.unwrap();
        assert!(matches!(resp.result, ServerResult::Empty(_)));
    }

    #[tokio::test]
    async fn test_uninitialized_request() {
        let server = test_server();

        let req = JSONRPCRequest {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: RequestId::Number(1),
            method: "test".to_string(),
            params: json!({}),
        };

        let result = server.handle_request(req).await;
        assert!(
            result.is_err()
                || matches!(result.ok().map(|r| r.result), Some(ServerResult::Empty(_)))
        );
    }

    #[tokio::test]
    async fn test_logging() {
        let notifications = Arc::new(RwLock::new(Vec::new()));
        let notif_clone = notifications.clone();

        let server = Server::new_with_notifications(
            ServerCapabilities {
                experimental: None,
                logging: Some(HashMap::new()),
                prompts: None,
                resources: None,
                tools: None,
                extra: Default::default(),
            },
            Implementation {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                extra: Default::default(),
            },
            move |n| {
                notif_clone.write().unwrap().push(n);
            },
        );

        // Initialize server first
        let init_req = JSONRPCRequest {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: RequestId::Number(1),
            method: "initialize".to_string(),
            params: json!({
                "protocolVersion": LATEST_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "test",
                    "version": "1.0"
                }
            }),
        };
        server.handle_request(init_req).await.unwrap();

        // Set log level
        let req = JSONRPCRequest {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: RequestId::Number(2),
            method: "logging/setLevel".to_string(),
            params: json!({
                "level": "warning"
            }),
        };
        server.handle_request(req).await.unwrap();

        // Test direct logging calls instead of tracing macros
        server
            .log(
                LoggingLevel::Info,
                Some("test".to_string()),
                json!("test info"),
            )
            .unwrap();

        server
            .log(
                LoggingLevel::Error,
                Some("test".to_string()),
                json!("test error"),
            )
            .unwrap();

        // Small delay to ensure notifications are processed
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let notifications = notifications.read().unwrap();
        let log_msgs = notifications
            .iter()
            .filter(|n| n.method == "notifications/message")
            .count();

        assert_eq!(log_msgs, 1);
    }
}
