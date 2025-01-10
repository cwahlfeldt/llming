use async_trait::async_trait;
use mcp_schema::{GetPromptResult, ListPromptsResult, Prompt, ServerResult};
use serde_json::Value;
use std::collections::HashMap;

use super::{HandlerState, SharedState};
use crate::handler::RequestHandler;
use crate::{Error, Result};

/// State for the prompts handler
#[derive(Default)]
pub(crate) struct PromptState {
    prompts: HashMap<String, Prompt>,
}

impl HandlerState for PromptState {}

/// Handler for prompt-related requests
#[derive(Clone)]
pub struct PromptHandler {
    state: SharedState<PromptState>,
}

impl Default for PromptHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptHandler {
    pub fn new() -> Self {
        Self {
            state: SharedState::default(),
        }
    }

    /// Register a prompt that can be used by clients
    pub fn register_prompt(&self, prompt: Prompt) -> Result<()> {
        let mut state = self
            .state
            .write()
            .map_err(|_| Error::Internal("state lock poisoned".into()))?;
        state.prompts.insert(prompt.name.clone(), prompt);
        Ok(())
    }
}

#[async_trait]
impl RequestHandler for PromptHandler {
    async fn handle(&self, params: Value) -> Result<ServerResult> {
        let method = params
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::InvalidParams("missing method".into()))?;

        match method {
            "prompts/list" => {
                let state = self
                    .state
                    .read()
                    .map_err(|_| Error::Internal("state lock poisoned".into()))?;
                let prompts = state.prompts.values().cloned().collect();
                Ok(ServerResult::ListPrompts(ListPromptsResult {
                    meta: None,
                    next_cursor: None,
                    prompts,
                    extra: Default::default(),
                }))
            }
            "prompts/get" => {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::InvalidParams("missing prompt name".into()))?;

                let state = self
                    .state
                    .read()
                    .map_err(|_| Error::Internal("state lock poisoned".into()))?;

                let prompt = state
                    .prompts
                    .get(name)
                    .ok_or_else(|| Error::InvalidRequest(format!("prompt not found: {}", name)))?;

                // Here we'd process any template arguments and return the rendered prompt
                // For now just return empty result
                Ok(ServerResult::GetPrompt(GetPromptResult {
                    meta: None,
                    description: prompt.description.clone(),
                    messages: Vec::new(),
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

    fn test_prompt() -> Prompt {
        Prompt {
            name: "test_prompt".into(),
            description: Some("A test prompt".into()),
            arguments: None,
            extra: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_prompt_list() {
        let handler = PromptHandler::new();
        handler.register_prompt(test_prompt()).unwrap();

        let result = handler
            .handle(json!({
                "method": "prompts/list",
            }))
            .await
            .unwrap();

        match result {
            ServerResult::ListPrompts(result) => {
                assert_eq!(result.prompts.len(), 1);
                assert_eq!(result.prompts[0].name, "test_prompt");
            }
            _ => panic!("Expected ListPrompts result"),
        }
    }
}
