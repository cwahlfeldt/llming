use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;
use mcp_schema::{
    Tool, ServerResult,
    ListToolsResult, CallToolResult,
};

use crate::{Result, Error};
use crate::handler::RequestHandler;
use super::{HandlerState, SharedState};

/// State for the tools handler
#[derive(Default)]
pub(crate) struct ToolState {
    tools: HashMap<String, Tool>,
    // Could add execution history, rate limiting, etc.
}

impl HandlerState for ToolState {}

/// Handler for tool-related requests
pub struct ToolHandler {
    state: SharedState<ToolState>,
}

impl ToolHandler {
    pub fn new() -> Self {
        Self {
            state: SharedState::default(),
        }
    }

    /// Register a tool that can be called by clients
    pub fn register_tool(&self, tool: Tool) -> Result<()> {
        let mut state = self.state.write().map_err(|_| Error::Internal("state lock poisoned".into()))?;
        state.tools.insert(tool.name.clone(), tool);
        Ok(())
    }
}

#[async_trait]
impl RequestHandler for ToolHandler {
    async fn handle(&self, params: Value) -> Result<ServerResult> {
        let method = params.get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::InvalidParams("missing method".into()))?;

        match method {
            "tools/list" => {
                let state = self.state.read().map_err(|_| Error::Internal("state lock poisoned".into()))?;
                let tools = state.tools.values().cloned().collect();
                Ok(ServerResult::ListTools(ListToolsResult {
                    meta: None,
                    next_cursor: None,
                    tools,
                    extra: Default::default(),
                }))
            }
            "tools/call" => {
                let name = params.get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::InvalidParams("missing tool name".into()))?;

                let state = self.state.read().map_err(|_| Error::Internal("state lock poisoned".into()))?;
                
                let tool = state.tools.get(name)
                    .ok_or_else(|| Error::InvalidRequest(format!("tool not found: {}", name)))?;

                // Here we'd actually execute the tool
                // For now just return empty result
                Ok(ServerResult::CallTool(CallToolResult {
                    meta: None,
                    content: Vec::new(),
                    is_error: None,
                    extra: Default::default(),
                }))
            }
            _ => Err(Error::MethodNotFound(method.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_tool() -> Tool {
        Tool {
            name: "test_tool".into(),
            description: Some("A test tool".into()),
            input_schema: mcp_schema::ToolInputSchema {
                type_: "object".into(),
                properties: None,
                required: None,
            },
            extra: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_tool_list() {
        let handler = ToolHandler::new();
        handler.register_tool(test_tool()).unwrap();

        let result = handler.handle(json!({
            "method": "tools/list",
        })).await.unwrap();

        match result {
            ServerResult::ListTools(result) => {
                assert_eq!(result.tools.len(), 1);
                assert_eq!(result.tools[0].name, "test_tool");
            }
            _ => panic!("Expected ListTools result"),
        }
    }
}
