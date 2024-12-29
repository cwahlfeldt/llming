use anyhow::Result;
use serde_json::{json, Value};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::info;

use super::client::{MCPFunction, MCPServerInfo, MCPTool};

pub async fn create_filesystem_mcp_server(
    addr: std::net::SocketAddr,
    allowed_paths: Vec<String>,
) -> Result<super::server::MCPServer> {
    info!(
        "Creating filesystem MCP server with allowed paths: {:?}",
        allowed_paths
    );

    // Create server info with filesystem tools
    let server_info = MCPServerInfo {
        name: "Filesystem MCP Server".to_string(),
        version: "1.0.0".to_string(),
        tools: vec![MCPTool {
            name: "files".to_string(),
            description: "File system operations".to_string(),
            functions: vec![
                MCPFunction {
                    name: "read_file".to_string(),
                    description: "Read contents of a file".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"}
                        },
                        "required": ["path"]
                    }),
                },
                MCPFunction {
                    name: "write_file".to_string(),
                    description: "Write content to a file".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"},
                            "content": {"type": "string"}
                        },
                        "required": ["path", "content"]
                    }),
                },
                MCPFunction {
                    name: "list_directory".to_string(),
                    description: "List contents of a directory".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"}
                        },
                        "required": ["path"]
                    }),
                },
                MCPFunction {
                    name: "search_files".to_string(),
                    description: "Search for files matching a pattern".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"},
                            "pattern": {"type": "string"}
                        },
                        "required": ["path", "pattern"]
                    }),
                },
            ],
        }],
        prompts: vec![],
    };

    let server = super::server::MCPServer::new(addr, server_info);
    let allowed_paths = Arc::new(allowed_paths);

    // Register file operations handlers
    let paths_clone = allowed_paths.clone();
    server
        .register_async_function("files", "read_file", move |params| {
            let paths_clone = paths_clone.clone();
            Box::pin(async move {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Path required"))?;

                if !is_path_allowed(path, &paths_clone) {
                    return Err(anyhow::anyhow!("Path not allowed: {}", path));
                }

                let content = fs::read_to_string(path).await?;
                Ok(json!({ "content": content }))
            })
        })
        .await;

    let paths_clone = allowed_paths.clone();
    server
        .register_async_function("files", "write_file", move |params| {
            let paths_clone = paths_clone.clone();
            Box::pin(async move {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Path required"))?;
                let content = params["content"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Content required"))?;

                if !is_path_allowed(path, &paths_clone) {
                    return Err(anyhow::anyhow!("Path not allowed: {}", path));
                }

                fs::write(path, content).await?;
                Ok(json!({ "success": true }))
            })
        })
        .await;

    let paths_clone = allowed_paths.clone();
    server
        .register_async_function("files", "list_directory", move |params| {
            let paths_clone = paths_clone.clone();
            Box::pin(async move {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Path required"))?;

                if !is_path_allowed(path, &paths_clone) {
                    return Err(anyhow::anyhow!("Path not allowed: {}", path));
                }

                let mut entries = Vec::new();
                let mut read_dir = fs::read_dir(path).await?;

                while let Some(entry) = read_dir.next_entry().await? {
                    let metadata = entry.metadata().await?;
                    entries.push(json!({
                        "name": entry.file_name().to_string_lossy(),
                        "path": entry.path().to_string_lossy(),
                        "is_file": metadata.is_file(),
                        "is_dir": metadata.is_dir(),
                        "size": metadata.len(),
                    }));
                }

                Ok(json!({
                    "path": path,
                    "entries": entries
                }))
            })
        })
        .await;

    let paths_clone = allowed_paths.clone();
    server
        .register_async_function("files", "search_files", move |params| {
            let paths_clone = paths_clone.clone();
            Box::pin(async move {
                let base_path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Path required"))?;
                let pattern = params["pattern"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Pattern required"))?;

                if !is_path_allowed(base_path, &paths_clone) {
                    return Err(anyhow::anyhow!("Path not allowed: {}", base_path));
                }

                let mut matches = Vec::new();
                search_files_recursive(base_path, pattern, &mut matches).await?;

                Ok(json!({
                    "base_path": base_path,
                    "pattern": pattern,
                    "matches": matches
                }))
            })
        })
        .await;

    Ok(server)
}

fn is_path_allowed(path: &str, allowed_paths: &[String]) -> bool {
    let path = PathBuf::from(path);
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If path doesn't exist yet (for write operations), check its parent
            if let Some(parent) = path.parent() {
                match parent.canonicalize() {
                    Ok(p) => p,
                    Err(_) => return false,
                }
            } else {
                return false;
            }
        }
    };

    for allowed in allowed_paths.iter() {
        let allowed_path = PathBuf::from(allowed);
        let canonical_allowed = match allowed_path.canonicalize() {
            Ok(p) => p,
            Err(_) => continue,
        };

        if canonical_path.starts_with(canonical_allowed) {
            return true;
        }
    }

    false
}

async fn search_files_recursive(
    base_path: &str,
    pattern: &str,
    matches: &mut Vec<Value>,
) -> Result<()> {
    let pattern = pattern.to_lowercase();
    let mut read_dir = fs::read_dir(base_path).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_lowercase();

        if name.contains(&pattern) {
            matches.push(json!({
                "name": path.file_name().unwrap().to_string_lossy(),
                "path": path.to_string_lossy(),
                "type": if metadata.is_dir() { "directory" } else { "file" },
                "size": metadata.len(),
                "is_file": metadata.is_file(),
                "is_dir": metadata.is_dir(),
            }));
        }

        if metadata.is_dir() {
            let future = Box::pin(search_files_recursive(
                path.to_str().unwrap(),
                &pattern,
                matches,
            ));
            future.await?;
        }
    }

    Ok(())
}
