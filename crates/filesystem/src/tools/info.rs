use super::{Tool, ToolExecute, ToolOutput};
use crate::security::PathValidator;
use mcp::protocol::{Annotated, Content, TextContent};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tokio::fs;

#[derive(Debug, Serialize)]
struct FileInfo {
    name: String,
    size: u64,
    created: String,
    modified: String,
    accessed: String,
    is_directory: bool,
    is_file: bool,
    permissions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symlink_target: Option<String>,
}

pub struct GetFileInfoTool {
    validator: Arc<PathValidator>,
}

impl GetFileInfoTool {
    pub fn new(validator: Arc<PathValidator>) -> Self {
        Self { validator }
    }

    async fn get_file_info(&self, path: &std::path::Path) -> crate::Result<FileInfo> {
        let metadata = if path.is_symlink() {
            fs::symlink_metadata(path).await?
        } else {
            fs::metadata(path).await?
        };

        // Format times with RFC3339 for ISO8601 compatibility
        let to_rfc3339 = |time: std::time::SystemTime| -> String {
            time.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "Invalid timestamp".to_string())
        };

        let symlink_target = if path.is_symlink() {
            std::fs::read_link(path)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        } else {
            None
        };

        Ok(FileInfo {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string()),
            size: metadata.len(),
            created: to_rfc3339(metadata.created().unwrap_or(std::time::UNIX_EPOCH)),
            modified: to_rfc3339(metadata.modified().unwrap_or(std::time::UNIX_EPOCH)),
            accessed: to_rfc3339(metadata.accessed().unwrap_or(std::time::UNIX_EPOCH)),
            is_directory: metadata.is_dir(),
            is_file: metadata.is_file(),
            permissions: format!("{:o}", metadata.permissions().mode() & 0o777),
            symlink_target,
        })
    }
}

impl Tool for GetFileInfoTool {
    fn name(&self) -> &'static str {
        "get_file_info"
    }

    fn description(&self) -> &'static str {
        "Retrieve detailed metadata about a file or directory. Returns comprehensive \
        information including size, creation time, last modified time, permissions, \
        and type. Special handling for symlinks to show their targets. All timestamps \
        are returned in RFC3339 format. Only works within allowed directories."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file or directory to get information about"
                }
            }
        })
    }
}

impl ToolExecute for GetFileInfoTool {
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

        // Validate path
        let path = self.validator.validate_path(path).await?;

        // Get file information
        let info = self.get_file_info(&path).await?;

        Ok(ToolOutput(vec![Content::Text(TextContent {
            text: serde_json::to_string_pretty(&info)?,
            annotated: Annotated { annotations: None },
        })]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_info() {
        let temp = tempdir().unwrap();
        let validator = Arc::new(PathValidator::new(vec![temp.path().to_path_buf()]));
        let tool = GetFileInfoTool::new(validator);

        // Create a test file
        let test_file = temp.path().join("test.txt");
        fs::write(&test_file, "test content").await.unwrap();

        // Get file info
        let params = HashMap::from([(
            "path".to_string(),
            json!(test_file.to_string_lossy().to_string()),
        )]);

        let result = tool.execute(Some(params)).await.unwrap();
        let content = &result[0];
        let info: serde_json::Value = serde_json::from_value(content.value.clone()).unwrap();

        // Verify basic fields
        assert_eq!(info["name"], "test.txt");
        assert_eq!(info["size"], 12); // "test content" length
        assert_eq!(info["is_file"], true);
        assert_eq!(info["is_directory"], false);
    }

    #[tokio::test]
    async fn test_directory_info() {
        let temp = tempdir().unwrap();
        let validator = Arc::new(PathValidator::new(vec![temp.path().to_path_buf()]));
        let tool = GetFileInfoTool::new(validator);

        // Get directory info
        let params = HashMap::from([(
            "path".to_string(),
            json!(temp.path().to_string_lossy().to_string()),
        )]);

        let result = tool.execute(Some(params)).await.unwrap();
        let content = &result[0];
        let info: serde_json::Value = serde_json::from_value(content.value.clone()).unwrap();

        // Verify it's a directory
        assert_eq!(info["is_directory"], true);
        assert_eq!(info["is_file"], false);
    }
}
