use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

// MCP Protocol Types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MCPFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MCPTool {
    pub name: String,
    pub description: String,
    pub functions: Vec<MCPFunction>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MCPPrompt {
    pub name: String,
    pub description: String,
    pub template: String,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MCPServerInfo {
    pub name: String,
    pub version: String,
    pub tools: Vec<MCPTool>,
    pub prompts: Vec<MCPPrompt>,
}

impl MCPServerInfo {
    pub fn new(
        name: String,
        version: String,
        tools: Vec<MCPTool>,
        prompts: Vec<MCPPrompt>,
    ) -> Self {
        Self {
            name,
            version,
            tools,
            prompts,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPFunctionCall {
    pub function: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPFunctionResult {
    pub result: serde_json::Value,
}

#[derive(Clone, Debug)]
pub struct MCPClient {
    http_client: crate::http::HttpClient,
    server_url: String,
    server_info: Option<MCPServerInfo>,
}

impl MCPClient {
    pub fn new(server_url: &str) -> Self {
        info!("Creating new MCP client for server: {}", server_url);
        Self {
            http_client: crate::http::HttpClient::new()
                .with_header("Content-Type", "application/json"),
            server_url: server_url.to_string(),
            server_info: None,
        }
    }

    pub async fn connect(&mut self) -> Result<&MCPServerInfo> {
        debug!("Attempting to connect to MCP server: {}", self.server_url);
        match self
            .http_client
            .send_request(
                hyper::Method::GET,
                &format!("{}/mcp/info", self.server_url),
                None::<()>,
            )
            .await
        {
            Ok(info) => {
                debug!("Successfully retrieved server info");
                self.server_info = Some(info);
                Ok(self.server_info.as_ref().unwrap())
            }
            Err(e) => {
                error!("Failed to connect to MCP server: {}", e);
                Err(anyhow::anyhow!("Connection failed: {}", e))
            }
        }
    }

    pub async fn call_function<T: Serialize + std::fmt::Debug>(
        &self,
        tool_name: &str,
        function_name: &str,
        parameters: T,
    ) -> Result<MCPFunctionResult> {
        debug!(
            "Calling function {}.{} with parameters: {:?}",
            tool_name, function_name, parameters
        );

        let call = MCPFunctionCall {
            function: format!("{}.{}", tool_name, function_name),
            parameters: serde_json::to_value(parameters)?,
        };

        match self
            .http_client
            .send_request(
                hyper::Method::POST,
                &format!("{}/mcp/function", self.server_url),
                Some(call),
            )
            .await
        {
            Ok(result) => {
                debug!("Function call successful");
                Ok(result)
            }
            Err(e) => {
                error!("Function call failed: {}", e);
                Err(anyhow::anyhow!("Function call failed: {}", e))
            }
        }
    }

    pub async fn get_prompt(&self, prompt_name: &str) -> Result<MCPPrompt> {
        debug!("Retrieving prompt: {}", prompt_name);
        self.http_client
            .send_request(
                hyper::Method::GET,
                &format!("{}/mcp/prompt/{}", self.server_url, prompt_name),
                None::<()>,
            )
            .await
            .map_err(|e| {
                error!("Failed to get prompt: {}", e);
                anyhow::anyhow!("Failed to get prompt: {}", e)
            })
    }

    pub async fn render_prompt<T: Serialize + std::fmt::Debug>(
        &self,
        prompt_name: &str,
        parameters: T,
    ) -> Result<String> {
        debug!(
            "Rendering prompt {} with parameters: {:?}",
            prompt_name, parameters
        );

        #[derive(Serialize)]
        struct RenderRequest<T> {
            parameters: T,
        }

        #[derive(Deserialize)]
        struct RenderResponse {
            rendered: String,
        }

        let response: RenderResponse = self
            .http_client
            .send_request(
                hyper::Method::POST,
                &format!("{}/mcp/prompt/{}/render", self.server_url, prompt_name),
                Some(RenderRequest { parameters }),
            )
            .await
            .map_err(|e| {
                error!("Failed to render prompt: {}", e);
                anyhow::anyhow!("Failed to render prompt: {}", e)
            })?;

        Ok(response.rendered)
    }
}
