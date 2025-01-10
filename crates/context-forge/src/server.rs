use mcp_schema::{
    Implementation, InitializeParams, InitializeResult, JSONRPCError, JSONRPCNotification,
    JSONRPCRequest, JSONRPCResponse, LoggingLevel, ProgressToken, RPCErrorDetail,
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
        let notification_sender = Arc::new(Box::new(notification_sender));

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
            let logging_handler = LoggingHandler::new(notification_sender.clone());
            server
                .register_handler("logging/setLevel", &logging_handler)
                .unwrap();
            server.logging_handler = Some(logging_handler);
        }

        // Initialize root handler if roots capability is enabled
        if let Some(roots) = &capabilities.roots {
            if roots.list_changed.unwrap_or(false) {
                let root_handler = RootHandler::new(Some(notification_sender.clone()));
                server
                    .register_handler("roots/list", &root_handler)
                    .unwrap();
                server.root_handler = Some(root_handler);
            } else {
                let root_handler = RootHandler::new(None);
                server
                    .register_handler("roots/list", &root_handler)
                    .unwrap();
                server.root_handler = Some(root_handler);
            }
        }

        // Initialize other handlers based on capabilities
        if capabilities.resources.is_some() {
            let handler = ResourceHandler::new();
            server.register_handler("resources/list", &handler).unwrap();
            server
                .register_handler("resources/templates/list", &handler)
                .unwrap();
            server.register_handler("resources/read", &handler).unwrap();
            server
                .register_handler("resources/subscribe", &handler)
                .unwrap();
            server
                .register_handler("resources/unsubscribe", &handler)
                .unwrap();
            server.resource_handler = Some(handler);
        }

        if capabilities.tools.is_some() {
            let handler = ToolHandler::new();
            server.register_handler("tools/list", &handler).unwrap();
            server.register_handler("tools/call", &handler).unwrap();
            server.tool_handler = Some(handler);
        }

        if capabilities.prompts.is_some() {
            let handler = PromptHandler::new();
            server.register_handler("prompts/list", &handler).unwrap();
            server.register_handler("prompts/get", &handler).unwrap();
            server.prompt_handler = Some(handler);
        }

        server
    }

    /// Create a new uninitialized server that discards notifications
    pub fn new(capabilities: ServerCapabilities, implementation: Implementation) -> Self {
        Self::new_with_notifications(capabilities, implementation, |_| {})
    }

    /// Register a request handler for a specific method
    pub fn register_handler<H>(&self, method: impl Into<String>, handler: &H) -> Result<()>
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
            ServerState::Initialized { .. } => {
                return Err(Error::AlreadyInitialized);
            }
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
    ) -> JSONRPCResponse<ServerResult> {
        let result = match request.method.as_str() {
            "initialize" => match serde_json::from_value(request.params) {
                Ok(params) => match self.handle_initialize(params).await {
                    Ok(result) => Ok(ServerResult::Initialize(result)),
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
                            obj.insert(
                                "_progress".to_string(),
                                serde_json::to_value(tracker).unwrap(),
                            );
                        }
                    }
                    handler.handle(params).await
                } else {
                    Err(Error::MethodNotFound(method.to_string()))
                }
            }
        };

        match result {
            Ok(result) => JSONRPCResponse {
                json_rpc: JSONRPC_VERSION.to_string(),
                id: request.id,
                result,
            },
            Err(e) => {
                let (code, message) = e.to_rpc_error();
                JSONRPCError {
                    json_rpc: JSONRPC_VERSION.to_string(),
                    id: request.id,
                    error: RPCErrorDetail {
                        code,
                        message,
                        data: None,
                    },
                }
                .into()
            }
        }
    }

    /// Handle an incoming JSON-RPC notification
    pub async fn handle_notification(
        &self,
        notification: JSONRPCNotification<Value>,
    ) -> Result<()> {
        // All notifications except initialized require initialization
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
impl From<JSONRPCError> for JSONRPCResponse<ServerResult> {
    fn from(error: JSONRPCError) -> Self {
        // This is a bit of a hack - we need to convert the error response into a regular response
        // We do this by serializing and deserializing since the types don't match directly
        serde_json::from_value(serde_json::to_value(error).unwrap()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        // Should fail before initialization
        let req = JSONRPCRequest {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: "1".into(),
            method: "test".to_string(),
            params: json!({}),
        };
        let resp = server.handle_request(req).await;
        assert!(matches!(resp, JSONRPCResponse { result: ServerResult::Empty(_), .. } if false));

        // Should succeed with valid initialize request
        let req = JSONRPCRequest {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: "2".into(),
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
        let resp = server.handle_request(req).await;
        match resp {
            JSONRPCResponse {
                result: ServerResult::Initialize(result),
                ..
            } => {
                assert_eq!(result.protocol_version, LATEST_PROTOCOL_VERSION);
            }
            _ => panic!("Expected initialize result"),
        }

        // Should fail if already initialized
        let req = JSONRPCRequest {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: "3".into(),
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
        let resp = server.handle_request(req).await;
        assert!(matches!(resp, JSONRPCResponse { result: ServerResult::Empty(_), .. } if false));
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
                // roots: None,
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

        // Initialize server
        let init_req = JSONRPCRequest {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: "1".into(),
            method: "initialize".to_string(),
            params: serde_json::json!({
                "protocolVersion": LATEST_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "test",
                    "version": "1.0"
                }
            }),
        };
        server.handle_request(init_req).await;

        // Test setting log level
        let req = JSONRPCRequest {
            json_rpc: JSONRPC_VERSION.to_string(),
            id: "2".into(),
            method: "logging/setLevel".to_string(),
            params: serde_json::json!({
                "level": "warning"
            }),
        };
        server.handle_request(req).await;

        // Test logging at different levels
        info!(server.logging_handler().unwrap(), "test info").unwrap();
        error!(server.logging_handler().unwrap(), "test error").unwrap();

        // Only error should be logged due to warning level
        let notifications = notifications.read().unwrap();
        let log_msgs = notifications
            .iter()
            .filter(|n| n.method == "notifications/message")
            .count();
        assert_eq!(log_msgs, 1);
    }
}
