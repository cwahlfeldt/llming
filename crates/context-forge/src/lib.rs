//! Context Forge is a toolkit for building Model Context Protocol (MCP) servers.
//!
//! It provides a simple and ergonomic way to create MCP-compatible servers with minimal
//! boilerplate while still allowing full control over the implementation details.

mod builder;
mod error;
mod handler;
mod handlers;
mod logging;
mod progress;
mod roots;
mod server;
mod transport;

pub use builder::ServerBuilder;
pub use error::{Error, Result};
pub use handler::{NotificationHandler, RequestHandler};
pub use progress::{get_progress_token, ProgressTracker};
pub use roots::RootHandler;
pub use server::Server;

// Export the default handlers
pub use handlers::{PromptHandler, ResourceHandler, ToolHandler};

// Export transport
pub use transport::stdio::StdioTransport;
pub use transport::{Message, Transport};

// Re-export main types from mcp-schema that users will need
pub use mcp_schema::{
    ClientNotification, ClientRequest, Implementation, ProgressToken, Prompt, Resource,
    ResourceTemplate, Root, ServerCapabilities, ServerResult, Tool,
};
