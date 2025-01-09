use mcp::protocol::{Annotated, TextContent};
use serde_json::json;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::fs;

use super::Tool;
use crate::security::PathValidator;

pub struct ListAllowedDirectoriesTool {
    validator: Arc<PathValidator>,
}

impl ListAllowedDirectoriesTool {
    pub fn new(validator: Arc<PathValidator>) -> Self {
        Self { validator }
    }
}

impl Tool for ListAllowedDirectoriesTool {
    fn name(&self) -> &'static str {
        "list_allowed_directories"
    }

    fn description(&self) -> &'static str {
        "Returns the list of directories that this server is allowed to access. \
        Use this to understand which directories are available before trying to access files. \
        The list shows canonicalized absolute paths where possible."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn run<'a>(
        &'a self,
        _params: Option<HashMap<String, serde_json::Value>>,
    ) -> Pin<Box<dyn Future<Output = crate::Result<TextContent>> + Send + 'a>> {
        Box::pin(async move {
            // Get allowed directories
            let mut dirs = Vec::new();
            for dir in self.validator.allowed_directories() {
                // Try to canonicalize the path, but fall back to the original if that fails
                let display_path = match fs::canonicalize(&dir).await {
                    Ok(p) => p,
                    Err(_) => dir.clone(),
                };

                // Add status info for each directory
                let status = match fs::metadata(&dir).await {
                    Ok(meta) if meta.is_dir() => "available",
                    Ok(_) => "not a directory",
                    Err(_) => "inaccessible",
                };

                dirs.push(json!({
                    "path": display_path.to_string_lossy(),
                    "status": status
                }));
            }

            let response = serde_json::json!({
                "allowed_directories": dirs,
                "note": "Paths shown are canonicalized where possible. Status indicates current accessibility."
            });

            Ok(TextContent {
                text: serde_json::to_string_pretty(&response)?,
                annotated: Annotated { annotations: None },
            })
        })
    }
}

// Remove this impl block entirely:
// impl ToolExecute for ListAllowedDirectoriesTool { ... }
