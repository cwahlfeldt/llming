use super::tools::Tool;
// use super::tools::ToolExecute;
use super::tools::*;
use crate::security::PathValidator;
use async_trait::async_trait;
use mcp::protocol::{messages::*, Implementation};
use mcp::{ModelContextServer, Result};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct FilesystemServer {
    validator: Arc<PathValidator>,
    tools: Vec<Box<dyn Tool>>,
}

impl FilesystemServer {
    pub fn new(allowed_dirs: Vec<PathBuf>) -> Self {
        let validator = Arc::new(PathValidator::new(allowed_dirs));

        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(ReadFileTool::new(validator.clone())),
            Box::new(ReadMultipleFilesTool::new(validator.clone())),
            Box::new(WriteFileTool::new(validator.clone())),
            Box::new(EditFileTool::new(validator.clone())),
            Box::new(ListDirectoryTool::new(validator.clone())),
            Box::new(CreateDirectoryTool::new(validator.clone())),
            Box::new(DirectoryTreeTool::new(validator.clone())),
            Box::new(MoveFileTool::new(validator.clone())),
            Box::new(SearchFilesTool::new(validator.clone())),
            Box::new(GetFileInfoTool::new(validator.clone())),
            Box::new(ListAllowedDirectoriesTool::new(validator.clone())),
        ];

        Self { validator, tools }
    }

    async fn handle_tool_call(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<ToolOutput> {
        // Convert arguments to HashMap if provided
        let args = arguments.map(|args| {
            args.as_object()
                .map(|obj| {
                    obj.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<HashMap<String, serde_json::Value>>()
                })
                .unwrap_or_default()
        });

        // Find and execute the tool
        for tool in &self.tools {
            if tool.name() == name {
                return Ok(tool.execute(args).await?);
            }
        }

        Err(mcp::Error::MethodNotFound(name.to_string()))
    }
}

#[async_trait]
impl ModelContextServer for FilesystemServer {
    async fn initialize(&self, params: InitializeRequest) -> Result<InitializeResult> {
        Ok(InitializeResult {
            protocol_version: params.protocol_version,
            server_info: Implementation {
                name: "filesystem-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: ServerCapabilities {
                tools: Some(json!({
                    // Tool capabilities are handled via the tools system
                })),
                ..Default::default()
            },
            instructions: Some("".to_string()),
        })
    }

    async fn handle_ping(&self) -> Result<()> {
        Ok(())
    }

    async fn create_message(&self, _: CreateMessageRequest) -> Result<CreateMessageResult> {
        Ok(CreateMessageResult {
            message: SamplingMessage {
                role: SamplingRole::Assistant,
                content: json!({
                    "kind": "text",
                    "value": "This server only handles filesystem operations."
                }),
            },
        })
    }

    async fn list_roots(&self) -> Result<ListRootsResult> {
        Ok(ListRootsResult { roots: vec![] })
    }

    async fn handle_roots_changed(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_cancelled(
        &self,
        _request_id: mcp::protocol::RequestId,
        _reason: Option<String>,
    ) -> Result<()> {
        Ok(())
    }

    async fn handle_progress(
        &self,
        _token: mcp::protocol::ProgressToken,
        _progress: f64,
        _total: Option<f64>,
    ) -> Result<()> {
        Ok(())
    }
}
