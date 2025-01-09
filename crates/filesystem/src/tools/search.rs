use mcp::protocol::{Annotated, Content, TextContent};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use super::{Tool, ToolOutput};
use crate::security::PathValidator;

pub struct SearchFilesTool {
    validator: Arc<PathValidator>,
}

impl SearchFilesTool {
    pub fn new(validator: Arc<PathValidator>) -> Self {
        Self { validator }
    }

    async fn search_directory(
        &self,
        dir: &std::path::Path,
        pattern: &str,
    ) -> crate::Result<Vec<String>> {
        let mut matches = vec![];
        let mut stack = vec![dir.to_path_buf()];
        let pattern = pattern.to_lowercase();

        while let Some(current_dir) = stack.pop() {
            let Ok(mut read_dir) = tokio::fs::read_dir(&current_dir).await else {
                continue;
            };

            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path = entry.path();
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();

                if name.contains(&pattern) {
                    matches.push(path.display().to_string());
                }

                if let Ok(metadata) = entry.metadata().await {
                    if metadata.is_dir() {
                        if let Ok(validated_path) = self.validator.validate_path(&path).await {
                            stack.push(validated_path);
                        }
                    }
                }
            }
        }

        Ok(matches)
    }
}

impl Tool for SearchFilesTool {
    fn name(&self) -> &'static str {
        "search_files"
    }

    fn description(&self) -> &'static str {
        "Recursively search for files and directories matching a pattern. \
        Searches through all subdirectories from the starting path. \
        Case-insensitive matching is used by default. \
        Only searches within allowed directories."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["path", "pattern"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory to start search from"
                },
                "pattern": {
                    "type": "string",
                    "description": "Pattern to search for in filenames"
                }
            }
        })
    }
}

impl ToolExecute for SearchFilesTool {
    async fn execute(
        &self,
        params: Option<HashMap<String, serde_json::Value>>,
    ) -> crate::Result<ToolOutput> {
        let params = params
            .ok_or_else(|| crate::error::Error::InvalidPath("No parameters provided".into()))?;

        let root = params
            .get("path")
            .ok_or_else(|| crate::error::Error::InvalidPath("No path provided".into()))?
            .as_str()
            .ok_or_else(|| crate::error::Error::InvalidPath("Path must be a string".into()))?;

        let pattern = params
            .get("pattern")
            .ok_or_else(|| crate::error::Error::InvalidPath("No pattern provided".into()))?
            .as_str()
            .ok_or_else(|| crate::error::Error::InvalidPath("Pattern must be a string".into()))?;

        // Validate and canonicalize root path
        let root = self.validator.validate_path(root).await?;

        let matches = self.search_directory(&root, pattern).await?;

        if matches.is_empty() {
            Ok(ToolOutput(vec![Content::Text(TextContent {
                text: "No matches found".to_string(),
                annotated: Annotated { annotations: None },
            })]))
        } else {
            Ok(ToolOutput(vec![Content::Text(TextContent {
                text: matches.join("\n").to_string(),
                annotated: Annotated { annotations: None },
            })]))
        }
    }
}
