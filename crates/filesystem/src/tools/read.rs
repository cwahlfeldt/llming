use super::{ReadFileRequest, Tool};
use crate::error::Error;
use crate::security::PathValidator;
use futures::future;
use mcp::protocol::common::{Annotated, TextContent};
use serde_json::json;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub struct ReadFileTool {
    validator: Arc<PathValidator>,
}

impl ReadFileTool {
    pub fn new(validator: Arc<PathValidator>) -> Self {
        Self { validator }
    }
}

impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read the complete contents of a file from the file system. \
        Handles various text encodings and provides detailed error messages \
        if the file cannot be read. Only works within allowed directories."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            }
        })
    }

    fn run<'a>(
        &'a self,
        params: Option<HashMap<String, serde_json::Value>>,
    ) -> Pin<Box<dyn Future<Output = crate::Result<TextContent>> + Send + 'a>> {
        Box::pin(async move {
            let request = ReadFileRequest::try_from(params)?;
            let path = self.validator.validate_path(&request.path).await?;
            let text = tokio::fs::read_to_string(path).await.map_err(Error::from)?;

            Ok(TextContent {
                text,
                annotated: Annotated { annotations: None },
            })
        })
    }
}

pub struct ReadMultipleFilesTool {
    validator: Arc<PathValidator>,
}

impl ReadMultipleFilesTool {
    pub fn new(validator: Arc<PathValidator>) -> Self {
        Self { validator }
    }

    async fn read_single_file(&self, path: String) -> String {
        match self.validator.validate_path(&path).await {
            Ok(validated_path) => match tokio::fs::read_to_string(&validated_path).await {
                Ok(content) => format!("{}:\n{}", path, content),
                Err(e) => format!("{}: Error reading file - {}", path, e),
            },
            Err(e) => format!("{}: Error validating path - {}", path, e),
        }
    }
}

impl Tool for ReadMultipleFilesTool {
    fn name(&self) -> &'static str {
        "read_multiple_files"
    }

    fn description(&self) -> &'static str {
        "Read the contents of multiple files simultaneously. \
        This is more efficient than reading files one by one when you need to analyze \
        or compare multiple files. Each file's content is returned with its path as \
        a reference. Failed reads for individual files won't stop the entire operation. \
        Only works within allowed directories."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["paths"],
            "properties": {
                "paths": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "description": "Path to a file to read"
                    },
                    "description": "List of file paths to read"
                }
            }
        })
    }

    fn run<'a>(
        &'a self,
        params: Option<HashMap<String, serde_json::Value>>,
    ) -> Pin<Box<dyn Future<Output = crate::Result<TextContent>> + Send + 'a>> {
        Box::pin(async move {
            let params = params
                .ok_or_else(|| crate::error::Error::InvalidPath("No parameters provided".into()))?;

            let paths = params
                .get("paths")
                .ok_or_else(|| crate::error::Error::InvalidPath("No paths provided".into()))?
                .as_array()
                .ok_or_else(|| crate::error::Error::InvalidPath("Paths must be an array".into()))?;

            let paths: Vec<String> = paths
                .iter()
                .filter_map(|p| p.as_str().map(|s| s.to_string()))
                .collect();

            if paths.is_empty() {
                return Ok(TextContent {
                    text: "No valid paths provided".to_string(),
                    annotated: Annotated { annotations: None },
                });
            }

            let results =
                future::join_all(paths.into_iter().map(|p| self.read_single_file(p))).await;

            Ok(TextContent {
                text: results.join("\n---\n"),
                annotated: Annotated { annotations: None },
            })
        })
    }
}
