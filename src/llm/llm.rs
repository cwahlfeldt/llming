use anyhow::Result;
use serde_json::Value;
use std::net::SocketAddr;
use tokio::time::Duration;
use tracing::{debug, error, info, warn};

use super::deepseek::DeepSeekClient;
use crate::mcp::*;

#[derive(Clone, Debug)]
pub struct LLM {
    llm: DeepSeekClient,
    mcp_client: MCPClient,
    mcp_server: MCPServer,
}

impl LLM {
    pub async fn new(addr: SocketAddr, allowed_paths: Vec<String>) -> Result<Self> {
        info!("Initializing LLM with mcp servers: [filesystem]");

        // Create and start the filesystem MCP server
        let mcp_server = create_filesystem_mcp_server(addr, allowed_paths).await?;

        // Start the server in a separate task and wait for it to be ready
        let (tx, rx) = tokio::sync::oneshot::channel();
        let server_handle = mcp_server.clone();

        tokio::spawn(async move {
            debug!("Server task starting");
            tx.send(()).expect("Failed to send server ready signal");

            if let Err(e) = server_handle.serve().await {
                error!("MCP server error: {}", e);
            }
        });

        // Wait for the server to signal it's ready
        rx.await?;
        debug!("Received server ready signal");

        // Add a small delay to ensure the server is fully up
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create client with explicit HTTP
        let mut mcp_client = MCPClient::new(&format!("http://{}", addr));

        // More robust connection retry logic
        let mut backoff = Duration::from_millis(100);
        let max_retries = 5;
        let mut attempt = 0;

        while attempt < max_retries {
            match mcp_client.connect().await {
                Ok(_) => {
                    info!("Successfully connected to MCP server");
                    break;
                }
                Err(e) => {
                    attempt += 1;
                    if attempt == max_retries {
                        error!("Failed to connect after {} attempts", max_retries);
                        return Err(anyhow::anyhow!(
                            "Failed to connect to MCP server after {} attempts: {}",
                            max_retries,
                            e
                        ));
                    }
                    warn!(
                        "Connection attempt {} failed, retrying in {:?}...",
                        attempt, backoff
                    );
                    tokio::time::sleep(backoff).await;
                    backoff *= 2; // Exponential backoff
                }
            }
        }

        let llm = DeepSeekClient::new();
        Ok(Self {
            llm,
            mcp_client,
            mcp_server,
        })
    }

    pub async fn chat(&self, message: &str) -> Result<String> {
        debug!("Processing chat message: {}", message);

        let mcp_tool_descriptions = serde_json::to_string(self.mcp_server.get_info());
        let system_prompt = r#"
            Model Context Server Information:
            You are an expert coding assistant with full access to the user's filesystem (/home/waffles) and will always search any directroies or files from /home/waffles. When users request file operations. You also act as a general chat interface and will answer any questions.
            "#;

        // First, send the combined prompt to the LLM
        debug!("Sending enhanced prompt to LLM");
        let enhanced_message = format!("{}{}", system_prompt, message);

        let initial_response = match self.llm.send_message(&enhanced_message).await {
            Ok(response) => response,
            Err(e) => {
                error!("LLM request failed: {}", e);
                return Err(anyhow::anyhow!("Failed to get response from LLM: {}", e));
            }
        };

        debug!("Got initial LLM response: {}", initial_response);

        // Extract and execute any file operations
        let operations = self.extract_operations(&initial_response);
        let mut final_response = initial_response.clone();

        if operations.is_empty() {
            debug!("No file operations found in response");
            return Ok(final_response);
        }

        debug!("Found {} operations to execute", operations.len());
        for op in &operations {
            debug!("Executing operation: {:?}", op);
            match self.execute_operation(op.clone()).await {
                Ok(result) => {
                    debug!("Operation succeeded: {:?}", result);

                    // Format the result in a markdown-friendly way
                    let result_str = match serde_json::to_string_pretty(&result) {
                        Ok(s) => s,
                        Err(e) => {
                            error!("Failed to format result: {}", e);
                            continue;
                        }
                    };

                    final_response.push_str("\n\n**Operation Result:**\n```json\n");
                    final_response.push_str(&result_str);
                    final_response.push_str("\n```\n");

                    // For complex results, get LLM to analyze them
                    if result_str.len() > 100 {
                        debug!("Getting LLM analysis of operation result");
                        let analysis_prompt = format!(
                            "Analyze this operation result and explain its key points:\n{}",
                            result_str
                        );

                        if let Ok(analysis) = self.llm.send_message(&analysis_prompt).await {
                            final_response.push_str("\n**Analysis:**\n");
                            final_response.push_str(&analysis);
                        }
                    }
                }
                Err(e) => {
                    error!("Operation failed: {}", e);
                    final_response.push_str("\n\n**Operation Failed:**\n```\n");
                    final_response.push_str(&e.to_string());
                    final_response.push_str("\n```\n");
                }
            }
        }

        // If we executed any operations, get a final summary from the LLM
        if !operations.is_empty() {
            debug!("Getting final summary from LLM");
            let summary_prompt = format!(
                    "Based on the file operations and their results above, provide a final summary and any relevant recommendations for the user's query: {}",
                    message
                );

            if let Ok(summary) = self.llm.send_message(&summary_prompt).await {
                final_response.push_str("\n\n**Summary:**\n");
                final_response.push_str(&summary);
            }
        }

        Ok(final_response)
    }

    async fn execute_operation(&self, operation: Operation) -> Result<serde_json::Value> {
        debug!(
            "Calling MCP function: {} with params: {:?}",
            operation.name, operation.parameters
        );

        match self
            .mcp_client
            .call_function("files", &operation.name, operation.parameters)
            .await
        {
            Ok(result) => {
                debug!("MCP function call succeeded");
                Ok(result.result)
            }
            Err(e) => {
                error!("MCP function call failed: {}", e);
                Err(anyhow::anyhow!("Function call failed: {}", e))
            }
        }
    }

    fn extract_operations(&self, response: &str) -> Vec<Operation> {
        let mut operations = Vec::new();
        let mut start_idx = 0;

        while let Some(start) = response[start_idx..].find("{{{") {
            if let Some(end) = response[start_idx + start..].find("}}}") {
                let json_str = &response[start_idx + start + 3..start_idx + start + end].trim();
                debug!("Extracted JSON string: {}", json_str);

                // First try parsing as-is
                let parse_result = serde_json::from_str::<Value>(json_str).or_else(|_| {
                    // If that fails, try some cleanup
                    let cleaned = json_str
                        .replace('\n', "")
                        .replace('\r', "")
                        .trim()
                        .to_string();
                    serde_json::from_str(&cleaned)
                });

                match parse_result {
                    Ok(value) => {
                        if let (Some(op), Some(params)) =
                            (value["operation"].as_str(), value.get("parameters"))
                        {
                            debug!("Parsed operation: {} with params: {:?}", op, params);
                            operations.push(Operation {
                                name: op.trim_start_matches("files.").to_string(),
                                parameters: params.clone(),
                            });
                        } else {
                            warn!("Missing operation or parameters in JSON: {}", json_str);
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse operation JSON: {} - Error: {}",
                            json_str, e
                        );
                        // If basic cleanup didn't work, try more aggressive fixing
                        if let Some(fixed_json) = try_fix_json(json_str) {
                            debug!("Attempting to parse fixed JSON: {}", fixed_json);
                            if let Ok(value) = serde_json::from_str::<Value>(&fixed_json) {
                                if let (Some(op), Some(params)) =
                                    (value["operation"].as_str(), value.get("parameters"))
                                {
                                    debug!("Successfully parsed fixed JSON");
                                    operations.push(Operation {
                                        name: op.trim_start_matches("files.").to_string(),
                                        parameters: params.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
                start_idx += start + end + 3;
            } else {
                break;
            }
        }

        debug!("Extracted {} operations from response", operations.len());
        operations
    }
}

fn try_fix_json(json_str: &str) -> Option<String> {
    let mut fixed = json_str.to_string();

    // Remove any leading/trailing whitespace
    fixed = fixed.trim().to_string();

    // Remove any extra whitespace between elements
    fixed = fixed.replace(": ", ":");
    fixed = fixed.replace(" :", ":");

    // Ensure proper quoting of keys
    for key in &["operation", "parameters", "path", "pattern", "content"] {
        fixed = fixed.replace(&format!("{}:", key), &format!("\"{}\":", key));
        fixed = fixed.replace(&format!("{} :", key), &format!("\"{}\":", key));
    }

    // Ensure proper quoting of string values
    if let Some(op_idx) = fixed.find("\"operation\":") {
        if let Some(colon_idx) = fixed[op_idx..].find(':') {
            let after_colon = &fixed[op_idx + colon_idx + 1..];
            if let Some(comma_idx) = after_colon.find(',') {
                let value = &after_colon[..comma_idx].trim();
                if !value.starts_with('"') {
                    fixed = fixed.replace(value, &format!("\"{}\"", value));
                }
            }
        }
    }

    // Ensure object has curly braces
    if !fixed.starts_with('{') {
        fixed = format!("{{{}", fixed);
    }
    if !fixed.ends_with('}') {
        fixed = format!("{}}}", fixed);
    }

    // Verify the result is valid JSON
    if serde_json::from_str::<Value>(&fixed).is_ok() {
        Some(fixed)
    } else {
        None
    }
}

#[derive(Debug, Clone)]
struct Operation {
    name: String,
    parameters: Value,
}
