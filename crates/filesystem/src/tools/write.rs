use mcp::protocol::{Annotated, Content, TextContent};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use super::{Tool, ToolExecute, ToolOutput};
use crate::security::PathValidator;

pub struct WriteFileTool {
    validator: Arc<PathValidator>,
}

impl WriteFileTool {
    pub fn new(validator: Arc<PathValidator>) -> Self {
        Self { validator }
    }
}

impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Create a new file or completely overwrite an existing file with new content. \
        Use with caution as it will overwrite existing files without warning. \
        Only works within allowed directories."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["path", "content"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path where to write the file"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            }
        })
    }
}

impl ToolExecute for WriteFileTool {
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

        let content = params
            .get("content")
            .ok_or_else(|| crate::error::Error::InvalidPath("No content provided".into()))?
            .as_str()
            .ok_or_else(|| crate::error::Error::InvalidPath("Content must be a string".into()))?;

        // Validate and canonicalize path
        let path = self.validator.validate_path(path).await?;

        // Write file contents
        tokio::fs::write(path, content).await?;

        Ok(ToolOutput(vec![Content::Text(TextContent {
            text: content.to_string(),
            annotated: Annotated { annotations: None },
        })]))
    }
}
