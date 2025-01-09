use mcp::protocol::{Annotated, Content, TextContent};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use super::{Tool, ToolExecute, ToolOutput};
use crate::security::PathValidator;

pub struct MoveFileTool {
    validator: Arc<PathValidator>,
}

impl MoveFileTool {
    pub fn new(validator: Arc<PathValidator>) -> Self {
        Self { validator }
    }
}

impl Tool for MoveFileTool {
    fn name(&self) -> &'static str {
        "move_file"
    }

    fn description(&self) -> &'static str {
        "Move or rename files and directories. Can move files between directories \
        and rename them in a single operation. If the destination exists, the \
        operation will fail. Works across different directories and can be used \
        for simple renaming within the same directory. Both source and destination \
        must be within allowed directories."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["source", "destination"],
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Source path of file or directory to move"
                },
                "destination": {
                    "type": "string",
                    "description": "Destination path where to move the file or directory"
                }
            }
        })
    }
}

impl ToolExecute for MoveFileTool {
    async fn execute(
        &self,
        params: Option<HashMap<String, serde_json::Value>>,
    ) -> crate::Result<ToolOutput> {
        let params = params
            .ok_or_else(|| crate::error::Error::InvalidPath("No parameters provided".into()))?;

        // Parse source and destination
        let source = params
            .get("source")
            .ok_or_else(|| crate::error::Error::InvalidPath("No source provided".into()))?
            .as_str()
            .ok_or_else(|| crate::error::Error::InvalidPath("Source must be a string".into()))?;

        let destination = params
            .get("destination")
            .ok_or_else(|| crate::error::Error::InvalidPath("No destination provided".into()))?
            .as_str()
            .ok_or_else(|| {
                crate::error::Error::InvalidPath("Destination must be a string".into())
            })?;

        // Validate both paths
        let source_path = self.validator.validate_path(source).await?;
        let dest_path = self.validator.validate_path(destination).await?;

        // Check if source exists
        if !source_path.exists() {
            return Err(crate::error::Error::InvalidPath(format!(
                "Source path does not exist: {}",
                source
            )));
        }

        // Check if destination already exists
        if dest_path.exists() {
            return Err(crate::error::Error::InvalidPath(format!(
                "Destination already exists: {}",
                destination
            )));
        }

        // Create parent directories if they don't exist
        if let Some(parent) = dest_path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        // Perform the move operation
        tokio::fs::rename(&source_path, &dest_path).await?;

        Ok(ToolOutput(vec![Content::Text(TextContent {
            text: format!(
                "Successfully moved '{}' to '{}'",
                source_path.display(),
                dest_path.display()
            ),
            annotated: Annotated { annotations: None },
        })]))
        // Content::Text(TextContent {
        //     text: content,
        //     annotated: Annotated { annotations: None },
        // })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_move_file() {
        let temp = tempdir().unwrap();
        let validator = Arc::new(PathValidator::new(vec![temp.path().to_path_buf()]));
        let tool = MoveFileTool::new(validator);

        // Create a test file
        let source = temp.path().join("source.txt");
        fs::write(&source, "test content").await.unwrap();

        // Setup move operation
        let dest = temp.path().join("dest.txt");
        let params = HashMap::from([
            (
                "source".to_string(),
                json!(source.to_string_lossy().to_string()),
            ),
            (
                "destination".to_string(),
                json!(dest.to_string_lossy().to_string()),
            ),
        ]);

        // Execute move
        let result = tool.execute(Some(params)).await.unwrap();

        // Verify
        assert!(!source.exists());
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).await.unwrap(), "test content");
    }

    #[tokio::test]
    async fn test_move_to_nested_directory() {
        let temp = tempdir().unwrap();
        let validator = Arc::new(PathValidator::new(vec![temp.path().to_path_buf()]));
        let tool = MoveFileTool::new(validator);

        // Create a test file
        let source = temp.path().join("source.txt");
        fs::write(&source, "test content").await.unwrap();

        // Setup move operation to nested directory
        let dest = temp.path().join("nested/path/dest.txt");
        let params = HashMap::from([
            (
                "source".to_string(),
                json!(source.to_string_lossy().to_string()),
            ),
            (
                "destination".to_string(),
                json!(dest.to_string_lossy().to_string()),
            ),
        ]);

        // Execute move
        let result = tool.execute(Some(params)).await.unwrap();

        // Verify
        assert!(!source.exists());
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).await.unwrap(), "test content");
    }
}
