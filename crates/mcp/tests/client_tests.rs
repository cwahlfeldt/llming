use async_trait::async_trait;
use mcp::{
    protocol::{common::*, constants::*, messages::*},
    transport::Transport,
    Error, MCPClient, Result,
};
use std::sync::{Arc, Mutex};

// Mock transport implementation for testing
struct MockTransport {
    sent_messages: Arc<Mutex<Vec<JSONRPCMessage>>>,
    responses: Arc<Mutex<Vec<JSONRPCMessage>>>,
}

impl MockTransport {
    fn new() -> Self {
        Self {
            sent_messages: Arc::new(Mutex::new(Vec::new())),
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn push_response(&self, response: JSONRPCMessage) {
        self.responses.lock().unwrap().push(response);
    }

    fn get_sent_messages(&self) -> Vec<JSONRPCMessage> {
        self.sent_messages.lock().unwrap().clone()
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn send(&self, message: JSONRPCMessage) -> Result<()> {
        self.sent_messages.lock().unwrap().push(message);
        Ok(())
    }

    async fn receive(&self) -> Result<JSONRPCMessage> {
        if let Some(response) = self.responses.lock().unwrap().pop() {
            Ok(response)
        } else {
            Err(Error::Protocol("No response available".into()))
        }
    }
}

// Helper function to create a test client
fn create_test_client() -> (MCPClient<MockTransport>, Arc<MockTransport>) {
    let transport = Arc::new(MockTransport::new());
    let client = MCPClient::builder()
        .transport(Arc::clone(&transport) as Arc<MockTransport>)
        .client_info(Implementation {
            name: "test-client".to_string(),
            version: "1.0.0".to_string(),
        })
        .capabilities(ClientCapabilities::default())
        .build()
        .unwrap();

    (client, transport)
}

#[tokio::test]
async fn test_client_builder() {
    // Test missing transport
    let result = MCPClient::<MockTransport>::builder()
        .client_info(Implementation {
            name: "test".to_string(),
            version: "1.0".to_string(),
        })
        .build();
    assert!(matches!(result, Err(Error::Internal(_))));

    // Test missing client info
    let transport = MockTransport::new();
    let result = MCPClient::builder().transport(transport).build();
    assert!(matches!(result, Err(Error::Internal(_))));

    // Test successful build
    let transport = MockTransport::new();
    let result = MCPClient::builder()
        .transport(transport)
        .client_info(Implementation {
            name: "test".to_string(),
            version: "1.0".to_string(),
        })
        .build();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_initialization() {
    let (client, transport) = create_test_client();

    // Setup mock response
    let server_info = Implementation {
        name: "test-server".to_string(),
        version: "1.0.0".to_string(),
    };
    let server_capabilities = ServerCapabilities::default();

    transport.push_response(JSONRPCMessage::Response(Response {
        id: 1,
        result: ResponseResult::Initialize(InitializeResult {
            protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
            server_info: server_info.clone(),
            capabilities: server_capabilities.clone(),
        }),
    }));

    // Test initialization
    let result = client.initialize().await;
    assert!(result.is_ok());

    // Verify sent message
    let sent_messages = transport.get_sent_messages();
    assert_eq!(sent_messages.len(), 1);
    match &sent_messages[0] {
        JSONRPCMessage::Request(request) => {
            assert_eq!(request.method, "initialize");
            match &request.params {
                Some(RequestParams::Initialize(params)) => {
                    assert_eq!(params.protocol_version, LATEST_PROTOCOL_VERSION);
                    assert_eq!(params.client_info.name, "test-client");
                }
                _ => panic!("Unexpected request params"),
            }
        }
        _ => panic!("Unexpected message type"),
    }

    // Verify client state
    assert_eq!(client.server_info().await, Some(server_info));
    assert_eq!(
        client.server_capabilities().await,
        Some(server_capabilities)
    );
}

#[tokio::test]
async fn test_ping() {
    let (client, transport) = create_test_client();

    // Test ping without initialization
    let result = client.ping().await;
    assert!(matches!(result, Err(Error::Protocol(_))));

    // Set initialized
    *client.initialized.write().await = true;

    // Setup mock response
    transport.push_response(JSONRPCMessage::Response(Response {
        id: 1,
        result: ResponseResult::Empty(()),
    }));

    // Test ping after initialization
    let result = client.ping().await;
    assert!(result.is_ok());

    // Verify sent message
    let sent_messages = transport.get_sent_messages();
    assert_eq!(sent_messages.len(), 1);
    match &sent_messages[0] {
        JSONRPCMessage::Request(request) => {
            assert_eq!(request.method, "ping");
            assert!(matches!(request.params, Some(RequestParams::Ping(_))));
        }
        _ => panic!("Unexpected message type"),
    }
}

#[tokio::test]
async fn test_list_resources() {
    let (client, transport) = create_test_client();

    // Set initialized
    *client.initialized.write().await = true;

    // Setup mock response
    let resources = vec![Resource {
        uri: "test://resource1".to_string(),
        name: "Resource 1".to_string(),
        resource_type: "test".to_string(),
        metadata: None,
    }];

    transport.push_response(JSONRPCMessage::Response(Response {
        id: 1,
        result: ResponseResult::ListResources(ListResourcesResult {
            resources: resources.clone(),
            cursor: None,
        }),
    }));

    // Test list_resources
    let result = client.list_resources(None).await;
    assert!(result.is_ok());
    let resources_result = result.unwrap();
    assert_eq!(resources_result.resources, resources);

    // Verify sent message
    let sent_messages = transport.get_sent_messages();
    assert_eq!(sent_messages.len(), 1);
    match &sent_messages[0] {
        JSONRPCMessage::Request(request) => {
            assert_eq!(request.method, "resources/list");
            assert!(matches!(
                request.params,
                Some(RequestParams::ListResources(_))
            ));
        }
        _ => panic!("Unexpected message type"),
    }
}

#[tokio::test]
async fn test_error_handling() {
    let (client, transport) = create_test_client();

    // Set initialized
    *client.initialized.write().await = true;

    // Test error response
    transport.push_response(JSONRPCMessage::Error(ErrorResponse {
        id: 1,
        error: JSONRPCError {
            code: -32000,
            message: "Test error".to_string(),
            data: None,
        },
    }));

    let result = client.ping().await;
    assert!(matches!(result, Err(Error::Protocol(_))));

    // Test unexpected message type
    transport.push_response(JSONRPCMessage::Notification(Notification {
        method: "test".to_string(),
        params: None,
    }));

    let result = client.ping().await;
    assert!(matches!(result, Err(Error::Protocol(_))));
}

#[tokio::test]
async fn test_notifications() {
    let (client, transport) = create_test_client();

    // Set initialized
    *client.initialized.write().await = true;

    // Test initialized notification
    let result = client.initialized().await;
    assert!(result.is_ok());

    // Verify sent message
    let sent_messages = transport.get_sent_messages();
    assert_eq!(sent_messages.len(), 1);
    match &sent_messages[0] {
        JSONRPCMessage::Notification(notification) => {
            assert_eq!(notification.method, "notifications/initialized");
            assert!(matches!(
                notification.params,
                Some(NotificationParams::Initialized(_))
            ));
        }
        _ => panic!("Unexpected message type"),
    }
}

#[tokio::test]
async fn test_resource_operations() {
    let (client, transport) = create_test_client();

    // Set initialized
    *client.initialized.write().await = true;

    // Test subscribe
    transport.push_response(JSONRPCMessage::Response(Response {
        id: 1,
        result: ResponseResult::Empty(()),
    }));

    let result = client.subscribe("test://resource").await;
    assert!(result.is_ok());

    // Test unsubscribe
    transport.push_response(JSONRPCMessage::Response(Response {
        id: 2,
        result: ResponseResult::Empty(()),
    }));

    let result = client.unsubscribe("test://resource").await;
    assert!(result.is_ok());

    // Verify sent messages
    let sent_messages = transport.get_sent_messages();
    assert_eq!(sent_messages.len(), 2);

    match &sent_messages[0] {
        JSONRPCMessage::Request(request) => {
            assert_eq!(request.method, "resources/subscribe");
            match &request.params {
                Some(RequestParams::Subscribe(params)) => {
                    assert_eq!(params.uri, "test://resource");
                }
                _ => panic!("Unexpected request params"),
            }
        }
        _ => panic!("Unexpected message type"),
    }

    match &sent_messages[1] {
        JSONRPCMessage::Request(request) => {
            assert_eq!(request.method, "resources/unsubscribe");
            match &request.params {
                Some(RequestParams::Unsubscribe(params)) => {
                    assert_eq!(params.uri, "test://resource");
                }
                _ => panic!("Unexpected request params"),
            }
        }
        _ => panic!("Unexpected message type"),
    }
}
