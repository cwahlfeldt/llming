use std::{env, path::PathBuf};
use mcp::transport::StdioTransport;
use filesystem::server::FilesystemServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get allowed directories from command line arguments
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: filesystem-server <allowed-directory> [additional-directories...]");
        std::process::exit(1);
    }

    // Convert arguments to absolute paths
    let allowed_dirs: Vec<PathBuf> = args.into_iter()
        .map(PathBuf::from)
        .map(|p| p.canonicalize().unwrap_or(p))
        .collect();

    // Create server instance with all tools
    let server = FilesystemServer::new(allowed_dirs.clone());

    // Create transport layer for stdio
    let transport = StdioTransport::new();

    // Create message handler
    let message_handler = mcp::server::MessageHandler::new(server);

    eprintln!("Filesystem MCP Server running...");
    eprintln!("Allowed directories:");
    for dir in allowed_dirs {
        eprintln!("  {}", dir.display());
    }

    // Process messages in a loop
    while let Ok(message) = transport.receive().await {
        if let Ok(Some(response)) = message_handler.handle_message(message).await {
            transport.send(response).await?;
        }
    }

    Ok(())
}