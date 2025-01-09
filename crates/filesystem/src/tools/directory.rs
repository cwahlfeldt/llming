use mcp::protocol::{Annotated, Content, TextContent};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::fs;

use super::{Tool, ToolExecute, ToolOutput};
use crate::security::PathValidator;

#[derive(Debug, Serialize)]
struct TreeEntry {
    name: String,
    #[serde(rename = "type")]
    entry_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<TreeEntry>>,
}

pub struct DirectoryTreeTool {
    validator: Arc<PathValidator>,
}

impl DirectoryTreeTool {
    pub fn new(validator: Arc<PathValidator>) -> Self {
        Self { validator }
    }

    async fn build_tree(&self, path: &std::path::Path) -> crate::Result<Vec<TreeEntry>> {
        self.build_tree_inner(path).await
    }

    fn build_tree_inner<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> Pin<Box<dyn Future<Output = crate::Result<Vec<TreeEntry>>> + Send + 'a>> {
        Box::pin(async move {
            let mut entries = Vec::new();
            let mut read_dir = fs::read_dir(path).await?;

            while let Some(entry) = read_dir.next_entry().await? {
                let metadata = entry.metadata().await?;
                let name = entry.file_name().to_string_lossy().to_string();

                let entry = if metadata.is_dir() {
                    match self.validator.validate_path(&entry.path()).await {
                        Ok(valid_path) => {
                            let children = self.build_tree_inner(&valid_path).await?;
                            TreeEntry {
                                name,
                                entry_type: "directory",
                                children: Some(children),
                            }
                        }
                        Err(_) => continue,
                    }
                } else {
                    TreeEntry {
                        name,
                        entry_type: "file",
                        children: None,
                    }
                };

                entries.push(entry);
            }

            entries.sort_by(|a, b| match (a.entry_type, b.entry_type) {
                ("directory", "file") => std::cmp::Ordering::Less,
                ("file", "directory") => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            });

            Ok(entries)
        })
    }
}

impl Tool for DirectoryTreeTool {
    fn name(&self) -> &'static str {
        "directory_tree"
    }

    fn description(&self) -> &'static str {
        "Get a recursive tree view of files and directories as a JSON structure. \
        Each entry includes 'name', 'type' (file/directory), and 'children' for directories. \
        Files have no children array, while directories always have a children array \
        (which may be empty). The output is ordered with directories first, then files, \
        both alphabetically. Only works within allowed directories."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to get tree structure for"
                }
            }
        })
    }
}

impl ToolExecute for DirectoryTreeTool {
    async fn execute(
        &self,
        params: Option<HashMap<String, serde_json::Value>>,
    ) -> crate::Result<ToolOutput> {
        let params = params
            .ok_or_else(|| crate::error::Error::InvalidPath("No parameters provided".into()))?;

        let path = params
            .get("path")
            .ok_or_else(|| crate::error::Error::InvalidPath("No path provided".into()))?
            .as_str()
            .ok_or_else(|| crate::error::Error::InvalidPath("Path must be a string".into()))?;

        let path = self.validator.validate_path(path).await?;

        let metadata = fs::metadata(&path).await?;
        if !metadata.is_dir() {
            return Err(crate::error::Error::InvalidPath(format!(
                "Path is not a directory: {}",
                path.display()
            )));
        }

        let tree = self.build_tree(&path).await?;

        Ok(ToolOutput(vec![Content::Text(TextContent {
            text: serde_json::to_string_pretty(&tree)?,
            annotated: Annotated { annotations: None },
        })]))
    }
}

pub struct ListDirectoryTool {
    validator: Arc<PathValidator>,
}

impl ListDirectoryTool {
    pub fn new(validator: Arc<PathValidator>) -> Self {
        Self { validator }
    }
}

impl Tool for ListDirectoryTool {
    fn name(&self) -> &'static str {
        "list_directory"
    }

    fn description(&self) -> &'static str {
        "Get a detailed listing of all files and directories in a specified path. \
        Results clearly distinguish between files and directories with [FILE] and [DIR] prefixes. \
        Only works within allowed directories."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list contents of"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: Option<HashMap<String, serde_json::Value>>,
    ) -> crate::Result<ToolOutput> {
        let params = params
            .ok_or_else(|| crate::error::Error::InvalidPath("No parameters provided".into()))?;

        let path = params
            .get("path")
            .ok_or_else(|| crate::error::Error::InvalidPath("No path provided".into()))?
            .as_str()
            .ok_or_else(|| crate::error::Error::InvalidPath("Path must be a string".into()))?;

        let path = self.validator.validate_path(path).await?;

        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(path).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let metadata = entry.metadata().await?;
            let prefix = if metadata.is_dir() { "[DIR]" } else { "[FILE]" };
            entries.push(format!(
                "{} {}",
                prefix,
                entry.file_name().to_string_lossy()
            ));
        }

        entries.sort();

        Ok(vec![Content {
            kind: "text".to_string(),
            value: json!(entries.join("\n")),
        }])
    }
}

pub struct CreateDirectoryTool {
    validator: Arc<PathValidator>,
}

impl CreateDirectoryTool {
    pub fn new(validator: Arc<PathValidator>) -> Self {
        Self { validator }
    }
}

impl Tool for CreateDirectoryTool {
    fn name(&self) -> &'static str {
        "create_directory"
    }

    fn description(&self) -> &'static str {
        "Create a new directory or ensure a directory exists. \
        Can create multiple nested directories in one operation. \
        If the directory already exists, this operation will succeed silently. \
        Only works within allowed directories."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to create"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: Option<HashMap<String, serde_json::Value>>,
    ) -> crate::Result<ToolOutput> {
        let params = params
            .ok_or_else(|| crate::error::Error::InvalidPath("No parameters provided".into()))?;

        let path = params
            .get("path")
            .ok_or_else(|| crate::error::Error::InvalidPath("No path provided".into()))?
            .as_str()
            .ok_or_else(|| crate::error::Error::InvalidPath("Path must be a string".into()))?;

        let path = self.validator.validate_path(path).await?;

        fs::create_dir_all(path).await?;

        Ok(vec![Content {
            kind: "text".to_string(),
            value: json!("Directory created successfully"),
        }])
    }
}
