use anyhow::Result;
use serde_json::{json, Value};
use std::future::Future;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tokio::fs;
use tracing::info;
use serde::Serialize;
use similar::{ChangeTag, TextDiff};
use std::time::SystemTime;
use super::super::client::{MCPFunction, MCPServerInfo, MCPTool};
use super::super::server::MCPServer;

#[derive(Debug, Serialize)]
struct FileInfo {
    size: u64,
    created: SystemTime,
    modified: SystemTime,
    accessed: SystemTime,
    is_directory: bool,
    is_file: bool,
    permissions: String,
}

#[derive(Debug)]
struct EditOperation {
    old_text: String,
    new_text: String,
}

#[derive(Debug, Serialize)]
struct TreeEntry {
    name: String,
    #[serde(rename = "type")]
    entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<TreeEntry>>,
}

pub async fn create_filesystem_mcp_server(
    addr: std::net::SocketAddr,
    allowed_paths: Vec<String>,
) -> Result<MCPServer> {
    info!(
        "Creating filesystem MCP server with allowed paths: {:?}",
        allowed_paths
    );

    let server = MCPServer::new(
        addr,
        MCPServerInfo {
            name: "filesystem-server".to_string(),
            version: "0.2.0".to_string(),
            tools: vec![MCPTool {
                name: "files".to_string(),
                description: "File system operations".to_string(),
                functions: get_tool_functions(),
            }],
            prompts: vec![],
        },
    );

    register_handlers(&server, &allowed_paths).await?;
    Ok(server)
}

fn get_tool_functions() -> Vec<MCPFunction> {
    vec![
        MCPFunction {
            name: "read_file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }),
            description:
                "Read the complete contents of a file from the file system. \
                Handles various text encodings and provides detailed error messages \
                if the file cannot be read. Use this tool when you need to examine \
                the contents of a single file. Only works within allowed directories."
                .to_string(),
        },
        MCPFunction {
            name: "write_file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"}
                },
                "required": ["path", "content"]
            }),
            description:
                    "Create a new file or completely overwrite an existing file with new content. \
                    Use with caution as it will overwrite existing files without warning. \
                    Handles text content with proper encoding. Only works within allowed directories."
                    .to_string(),
        },
        MCPFunction {
            name: "edit_file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "edits": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "oldText": {"type": "string"},
                                "newText": {"type": "string"}
                            }
                        }
                    },
                    "dryRun": {"type": "boolean"}
                },
                "required": ["path", "edits"]
            }),
            description:
                "Make line-based edits to a text file. Each edit replaces exact line sequences \
                with new content. Returns a git-style diff showing the changes made. \
                Only works within allowed directories.".to_string(),
        },
        MCPFunction {
            name: "create_directory".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }),
            description: 
                "Create a new directory or ensure a directory exists. Can create multiple \
                nested directories in one operation. If the directory already exists, \
                this operation will succeed silently. Perfect for setting up directory \
                structures for projects or ensuring required paths exist. Only works within allowed directories."
                .to_string(),
        },
        MCPFunction {
            name: "list_directory".to_string(),
            description: "Get a detailed listing of all files and directories in a specified path. Results clearly distinguish between files and directories with [FILE] and [DIR] prefixes. This tool is essential for understanding directory structure and finding specific files within a directory. Only works within allowed directories.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }),
        },
        MCPFunction {
            name: "directory_tree".to_string(),
            description: "Get a recursive tree view of files and directories as a JSON structure. Each entry includes 'name', 'type' (file/directory), and 'children' for directories. Files have no children array, while directories always have a children array (which may be empty). The output is formatted with 2-space indentation for readability. Only works within allowed directories.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }),
        },
        MCPFunction {
            name: "move_file".to_string(),
            description: "Move or rename files and directories. Can move files between directories and rename them in a single operation. If the destination exists, the operation will fail. Works across different directories and can be used for simple renaming within the same directory. Both source and destination must be within allowed directories.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "source": {"type": "string"},
                    "destination": {"type": "string"}
                },
                "required": ["source", "destination"]
            }),
        },
        MCPFunction {
            name: "search_files".to_string(),
            description: "Recursively search for files and directories matching a pattern. Searches through all subdirectories from the starting path. The search is case-insensitive and matches partial names. Returns full paths to all matching items. Great for finding files when you don't know their exact location. Only searches within allowed directories.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "pattern": {"type": "string"}
                },
                "required": ["path", "pattern"]
            }),
        },
        MCPFunction {
            name: "get_file_info".to_string(),
            description: "Retrieve detailed metadata about a file or directory. Returns comprehensive information including size, creation time, last modified time, permissions, and type. This tool is perfect for understanding file characteristics without reading the actual content. Only works within allowed directories.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }),
        },
        MCPFunction {
            name: "list_allowed_directories".to_string(),
            description: "Returns the list of directories that this server is allowed to access. Use this to understand which directories are available before trying to access files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    ]
}

async fn register_handlers(server: &MCPServer, allowed_paths: &[String]) -> Result<()> {
    let paths_clone = allowed_paths.to_vec();

    // Read file handler
    server
        .register_async_function("files", "read_file", move |params| {
            let paths = paths_clone.clone();
            Box::pin(async move {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Path required"))?;

                if !is_path_allowed(path, &paths) {
                    return Err(anyhow::anyhow!("Path not allowed"));
                }

                let content = fs::read_to_string(path).await?;
                Ok(json!({ "content": content }))
            })
        })
        .await;

    // Write file handler
    let paths_clone = allowed_paths.to_vec();
    server
        .register_async_function("files", "write_file", move |params| {
            let paths = paths_clone.clone();
            Box::pin(async move {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Path required"))?;
                let content = params["content"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Content required"))?;

                if !is_path_allowed(path, &paths) {
                    return Err(anyhow::anyhow!("Path not allowed"));
                }

                fs::write(path, content).await?;
                Ok(json!({ "success": true }))
            })
        })
        .await;

    // Edit file handler
    let paths_clone = allowed_paths.to_vec();
    server
        .register_async_function("files", "edit_file", move |params| {
            let paths = paths_clone.clone();
            Box::pin(async move {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Path required"))?;
                let edits = params["edits"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Edits required"))?;
                let dry_run = params["dryRun"].as_bool().unwrap_or(false);

                if !is_path_allowed(path, &paths) {
                    return Err(anyhow::anyhow!("Path not allowed"));
                }

                let edit_ops: Vec<EditOperation> = edits
                    .iter()
                    .map(|edit| EditOperation {
                        old_text: edit["oldText"].as_str().unwrap_or("").to_string(),
                        new_text: edit["newText"].as_str().unwrap_or("").to_string(),
                    })
                    .collect();

                let diff = apply_file_edits(path, &edit_ops, dry_run).await?;
                Ok(json!({ "diff": diff }))
            })
        })
        .await;

    // Create directory handler
    let paths_clone = allowed_paths.to_vec();
    server
        .register_async_function("files", "create_directory", move |params| {
            let paths = paths_clone.clone();
            Box::pin(async move {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Path required"))?;

                if !is_path_allowed(path, &paths) {
                    return Err(anyhow::anyhow!("Path not allowed"));
                }

                fs::create_dir_all(path).await?;
                Ok(json!({ "success": true }))
            })
        })
        .await;

    // List directory handler
    let paths_clone = allowed_paths.to_vec();
    server
        .register_async_function("files", "list_directory", move |params| {
            let paths = paths_clone.clone();
            Box::pin(async move {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Path required"))?;

                if !is_path_allowed(path, &paths) {
                    return Err(anyhow::anyhow!("Path not allowed"));
                }

                let mut entries = Vec::new();
                let mut dir = fs::read_dir(path).await?;
                while let Some(entry) = dir.next_entry().await? {
                    let file_type = if entry.file_type().await?.is_dir() {
                        "directory"
                    } else {
                        "file"
                    };
                    entries.push(json!({
                        "name": entry.file_name().to_string_lossy(),
                        "type": file_type
                    }));
                }
                Ok(json!({ "entries": entries }))
            })
        })
        .await;

    // Directory tree handler
    let paths_clone = allowed_paths.to_vec();
    server
        .register_async_function("files", "directory_tree", move |params| {
            let paths = paths_clone.clone();
            Box::pin(async move {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Path required"))?;

                if !is_path_allowed(path, &paths) {
                    return Err(anyhow::anyhow!("Path not allowed"));
                }

                fn build_tree(path: &str) -> impl Future<Output = Result<Vec<TreeEntry>>> + '_ {
                    Box::pin(async move {
                        let mut entries = Vec::new();
                        let mut dir = fs::read_dir(path).await?;
                        while let Some(entry) = dir.next_entry().await? {
                            let name = entry.file_name().to_string_lossy().into_owned();
                            if entry.file_type().await?.is_dir() {
                                entries.push(TreeEntry {
                                    name,
                                    entry_type: "directory".to_string(),
                                    children: Some(
                                        build_tree(&entry.path().to_string_lossy()).await?,
                                    ),
                                });
                            } else {
                                entries.push(TreeEntry {
                                    name,
                                    entry_type: "file".to_string(),
                                    children: None,
                                });
                            }
                        }
                        Ok(entries)
                    })
                }

                let tree = build_tree(path).await?;
                Ok(json!({ "tree": tree }))
            })
        })
        .await;

    // Move file handler
    let paths_clone = allowed_paths.to_vec();
    server
        .register_async_function("files", "move_file", move |params| {
            let paths = paths_clone.clone();
            Box::pin(async move {
                let source = params["source"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Source required"))?;
                let destination = params["destination"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Destination required"))?;

                if !is_path_allowed(source, &paths) || !is_path_allowed(destination, &paths) {
                    return Err(anyhow::anyhow!("Path not allowed"));
                }

                fs::rename(source, destination).await?;
                Ok(json!({ "success": true }))
            })
        })
        .await;

    // Search files handler
    let paths_clone = allowed_paths.to_vec();
    server
        .register_async_function("files", "search_files", move |params| {
            let paths = paths_clone.clone();
            Box::pin(async move {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Path required"))?;
                let pattern = params["pattern"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Pattern required"))?;

                if !is_path_allowed(path, &paths) {
                    return Err(anyhow::anyhow!("Path not allowed"));
                }

                let mut matches = Vec::new();
                search_files_recursive(path, pattern, &mut matches).await?;
                Ok(json!({ "matches": matches }))
            })
        })
        .await;

    // Get file info handler
    let paths_clone = allowed_paths.to_vec();
    server
        .register_async_function("files", "get_file_info", move |params| {
            let paths = paths_clone.clone();
            Box::pin(async move {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Path required"))?;

                if !is_path_allowed(path, &paths) {
                    return Err(anyhow::anyhow!("Path not allowed"));
                }

                let metadata = fs::metadata(path).await?;
                let info = FileInfo {
                    size: metadata.len(),
                    created: metadata.created()?,
                    modified: metadata.modified()?,
                    accessed: metadata.accessed()?,
                    is_directory: metadata.is_dir(),
                    is_file: metadata.is_file(),
                    permissions: format!("{:o}", metadata.permissions().mode() & 0o777),
                };
                Ok(json!(info))
            })
        })
        .await;

    Ok(())
}

fn is_path_allowed(path: &str, allowed_paths: &[String]) -> bool {
    let path = PathBuf::from(path);
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
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

    for allowed in allowed_paths {
        let allowed_path = PathBuf::from(allowed);
        if let Ok(canonical_allowed) = allowed_path.canonicalize() {
            if canonical_path.starts_with(canonical_allowed) {
                return true;
            }
        }
    }
    false
}

fn search_files_recursive<'a>(
    base_path: &'a str,
    pattern: &'a str,
    matches: &'a mut Vec<Value>,
) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        let mut dir = fs::read_dir(base_path).await?;
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if entry.file_type().await?.is_dir() {
                search_files_recursive(&path.to_string_lossy(), pattern, matches).await?;
            } else if path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
                .contains(&pattern.to_lowercase())
            {
                matches.push(json!(path.to_string_lossy()));
            }
        }
        Ok(())
    })
}

async fn apply_file_edits(path: &str, edits: &[EditOperation], dry_run: bool) -> Result<String> {
    let content = fs::read_to_string(path).await?;
    let mut modified = content.clone();

    for edit in edits {
        if modified.contains(&edit.old_text) {
            modified = modified.replace(&edit.old_text, &edit.new_text);
        } else {
            return Err(anyhow::anyhow!("Could not find text to replace"));
        }
    }

    let diff = TextDiff::from_lines(&content, &modified);
    let diff_output = diff
        .iter_all_changes()
        .map(|change| {
            let prefix = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            format!("{}{}", prefix, change)
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !dry_run {
        fs::write(path, modified).await?;
    }

    Ok(diff_output)
}
