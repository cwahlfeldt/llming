# Context Forge

A Rust implementation of the [Model Context Protocol (MCP)](https://modelcontextprotocol.github.io/specification), providing a toolkit for building MCP-compatible servers.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- 🚀 Full MCP specification implementation
- 🔒 Thread-safe by design
- 🔌 Pluggable transport layer (stdio, HTTP)
- 📝 Built-in resource management
- 🛠 Extensible tool support
- 📋 Prompt template system
- 📊 Progress tracking
- 📡 Root management
- 📝 Logging system

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
context-forge = { git = "https://github.com/your-username/context-forge" }
```

## Quick Start

Here's a simple example of creating an MCP server:

```rust
use context_forge::{Server, ServerBuilder, Result};
use mcp_schema::{Resource, Tool};

#[tokio::main]
async fn main() -> Result<()> {
    // Create server with basic capabilities
    let server = ServerBuilder::new("my-server", "1.0")
        .with_resources(true, true)  // Enable resources with subscribe and list_changed
        .with_tools(true)           // Enable tools with list_changed
        .with_logging()             // Enable logging
        .build()?;

    // Register resources
    if let Some(handler) = server.resource_handler() {
        handler.register_resource(Resource {
            uri: "file:///data/config.json".into(),
            name: "Configuration".into(),
            description: Some("System configuration file".into()),
            mime_type: Some("application/json".into()),
            annotated: Default::default(),
        })?;
    }

    // Use stdio transport
    let mut transport = StdioTransport::new();

    // Run the server
    transport.run(|msg| async {
        match msg {
            Message::Request(req) => {
                let response = server.handle_request(req).await;
                Ok(Some(Message::Response(response)))
            }
            Message::Notification(notif) => {
                server.handle_notification(notif).await?;
                Ok(None)
            }
            _ => Err(Error::InvalidRequest("unexpected message type".into())),
        }
    }).await?;

    Ok(())
}
```

## Advanced Usage

### Custom Resource Handler

```rust
use context_forge::{RequestHandler, Result, ServerResult};
use async_trait::async_trait;
use serde_json::Value;

struct CustomResourceHandler {
    // Your handler state
}

#[async_trait]
impl RequestHandler for CustomResourceHandler {
    async fn handle(&self, params: Value) -> Result<ServerResult> {
        // Handle resource requests
        match params.get("method").and_then(|v| v.as_str()) {
            Some("resources/list") => {
                // Return list of resources
            }
            Some("resources/read") => {
                // Read resource content
            }
            _ => Err(Error::MethodNotFound("unknown method".into()))
        }
    }
}
```

### Progress Tracking

```rust
async fn long_running_task(server: &Server) -> Result<()> {
    // Create progress tracker
    let tracker = server.create_progress_tracker(
        ProgressToken::Number(1),
        Some(100.0)
    );

    // Update progress
    tracker.update(25.0);
    // Do some work...
    tracker.update(50.0);
    // More work...
    tracker.update(100.0);

    Ok(())
}
```

### Logging

```rust
// Using log macros
if let Some(handler) = server.logging_handler() {
    info!(handler, "Starting operation")?;
    warn!(handler, "Resource {} not found", id)?;
    error!(handler, "Failed to process request: {}", err)?;
}
```

## Features in Detail

### Resource Management
- File system resources
- Memory resources
- Resource templates
- Subscription support

### Tool System
- Dynamic tool registration
- Tool execution
- Result handling

### Prompt System
- Template support
- Argument validation
- Prompt rendering

### Progress Tracking
- Progress notifications
- Event streaming
- Cancellation support

### Root Management
- File system roots
- Custom root providers
- Root change notifications

### Logging System
- Multiple log levels
- Structured logging
- Log filtering

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.


```
I am working on implementing the Model Context Protocol (MCP) specification in Rust. We have a project called 'context-forge' located in /var/home/waffles/code/llming/ that is implementing this specification. Key progress so far:

1. Implemented core MCP schema types in mcp-schema crate
2. Created basic server structure with:
   - Initialization handling
   - Request/notification routing
   - Progress notifications
   - Root management
   - Logging system

Currently working on: [CURRENT_TASK]

The project has these key files and crates:
- mcp-schema: Core MCP types and definitions
- hyperax: HTTP client/server library we might use later for transport
- context-forge: Main implementation containing:
  - server.rs: Core server implementation
  - handler.rs: Request/notification handler traits
  - handlers/: Individual handler implementations
  - progress.rs: Progress tracking
  - roots.rs: Root management
  - logging.rs: Logging system

Please look at the files in /var/home/waffles/code/llming/crates/ to understand the current implementation and help continue the development.

What remaining features from the MCP specification should we implement next?
```