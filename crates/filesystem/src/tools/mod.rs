use crate::error::Result;
use async_trait::async_trait;
use mcp::protocol::{Annotated, Content, TextContent};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, pin::Pin};

mod allowed;
mod directory;
mod edit;
mod info;
mod move_files;
mod read;
mod search;
mod write;

pub use allowed::*;
pub use directory::*;
pub use edit::*;
pub use info::*;
pub use move_files::*;
pub use read::*;
pub use search::*;
pub use write::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadFileRequest {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadMultipleFilesRequest {
    pub paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WriteFileRequest {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EditOperation {
    pub old_text: String,
    pub new_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EditFileRequest {
    pub path: String,
    pub edits: Vec<EditOperation>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MoveFileRequest {
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListDirectoryRequest {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDirectoryRequest {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchFilesRequest {
    pub path: String,
    pub pattern: String,
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetFileInfoRequest {
    pub path: String,
}

#[derive(Debug)]
pub struct ToolOutput(pub Vec<mcp::protocol::Content>);

pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn input_schema(&self) -> serde_json::Value;

    // Core method with the specific implementation
    fn run<'a>(
        &'a self,
        params: Option<HashMap<String, serde_json::Value>>,
    ) -> Pin<Box<dyn Future<Output = crate::Result<TextContent>> + Send + 'a>>;

    // Default implementation that handles MCP wrapping
    fn execute<'a>(
        &'a self,
        params: Option<HashMap<String, serde_json::Value>>,
    ) -> Pin<Box<dyn Future<Output = crate::Result<ToolOutput>> + Send + 'a>> {
        Box::pin(async move {
            let content = self.run(params).await?;
            Ok(ToolOutput(vec![Content::Text(content)]))
        })
    }
}

// pub trait ToolExecute {
//     async fn execute(
//         &self,
//         params: Option<HashMap<String, serde_json::Value>>,
//     ) -> std::result::Result<ToolOutput, crate::error::Error>;
// }

impl TryFrom<Option<HashMap<String, serde_json::Value>>> for ReadFileRequest {
    type Error = crate::error::Error;

    fn try_from(params: Option<HashMap<String, serde_json::Value>>) -> Result<Self> {
        let params = params
            .ok_or_else(|| crate::error::Error::InvalidPath("No parameters provided".into()))?;
        let path = params
            .get("path")
            .ok_or_else(|| crate::error::Error::InvalidPath("No path provided".into()))?
            .as_str()
            .ok_or_else(|| crate::error::Error::InvalidPath("Path must be a string".into()))?;

        Ok(ReadFileRequest {
            path: path.to_string(),
        })
    }
}

impl TryFrom<Option<HashMap<String, serde_json::Value>>> for ReadMultipleFilesRequest {
    type Error = crate::error::Error;

    fn try_from(params: Option<HashMap<String, serde_json::Value>>) -> Result<Self> {
        let params = params
            .ok_or_else(|| crate::error::Error::InvalidPath("No parameters provided".into()))?;
        let paths = params
            .get("paths")
            .ok_or_else(|| crate::error::Error::InvalidPath("No paths provided".into()))?
            .as_array()
            .ok_or_else(|| crate::error::Error::InvalidPath("Paths must be an array".into()))?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| crate::error::Error::InvalidPath("Path must be a string".into()))
                    .map(|s| s.to_string())
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(ReadMultipleFilesRequest { paths })
    }
}

impl TryFrom<Option<HashMap<String, serde_json::Value>>> for WriteFileRequest {
    type Error = crate::error::Error;

    fn try_from(params: Option<HashMap<String, serde_json::Value>>) -> Result<Self> {
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

        Ok(WriteFileRequest {
            path: path.to_string(),
            content: content.to_string(),
        })
    }
}

impl TryFrom<Option<HashMap<String, serde_json::Value>>> for SearchFilesRequest {
    type Error = crate::error::Error;

    fn try_from(params: Option<HashMap<String, serde_json::Value>>) -> Result<Self> {
        let params = params
            .ok_or_else(|| crate::error::Error::InvalidPath("No parameters provided".into()))?;
        let path = params
            .get("path")
            .ok_or_else(|| crate::error::Error::InvalidPath("No path provided".into()))?
            .as_str()
            .ok_or_else(|| crate::error::Error::InvalidPath("Path must be a string".into()))?;
        let pattern = params
            .get("pattern")
            .ok_or_else(|| crate::error::Error::InvalidPath("No pattern provided".into()))?
            .as_str()
            .ok_or_else(|| crate::error::Error::InvalidPath("Pattern must be a string".into()))?;

        let exclude_patterns = params
            .get("exclude_patterns")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(SearchFilesRequest {
            path: path.to_string(),
            pattern: pattern.to_string(),
            exclude_patterns,
        })
    }
}

impl TryFrom<Option<HashMap<String, serde_json::Value>>> for MoveFileRequest {
    type Error = crate::error::Error;

    fn try_from(params: Option<HashMap<String, serde_json::Value>>) -> Result<Self> {
        let params = params
            .ok_or_else(|| crate::error::Error::InvalidPath("No parameters provided".into()))?;
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

        Ok(MoveFileRequest {
            source: source.to_string(),
            destination: destination.to_string(),
        })
    }
}
