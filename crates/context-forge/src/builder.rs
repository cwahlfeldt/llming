use std::collections::HashMap;
use mcp_schema::{
    ServerCapabilities, Implementation,
    ResourcesCapability, ToolsCapability, PromptsCapability,
    Resource, Tool, Prompt,
};

use crate::{Server, Result, Error};

/// Builder pattern for configuring and creating an MCP server.
pub struct ServerBuilder {
    capabilities: ServerCapabilities,
    implementation: Implementation,
    resources: Vec<Resource>,
    tools: Vec<Tool>,
    prompts: Vec<Prompt>,
}

impl ServerBuilder {
    /// Create a new server builder with minimal defaults.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            capabilities: ServerCapabilities {
                experimental: None,
                logging: None,
                prompts: None,
                resources: None,
                tools: None,
                extra: HashMap::new(),
            },
            implementation: Implementation {
                name: name.into(),
                version: version.into(),
                extra: HashMap::new(),
            },
            resources: Vec::new(),
            tools: Vec::new(),
            prompts: Vec::new(),
        }
    }

    /// Enable resources support with the given capabilities.
    pub fn with_resources(mut self, subscribe: bool, list_changed: bool) -> Self {
        self.capabilities.resources = Some(ResourcesCapability {
            subscribe: Some(subscribe),
            list_changed: Some(list_changed),
        });
        self
    }

    /// Enable tools support.
    pub fn with_tools(mut self, list_changed: bool) -> Self {
        self.capabilities.tools = Some(ToolsCapability {
            list_changed: Some(list_changed),
        });
        self
    }

    /// Enable prompts support.
    pub fn with_prompts(mut self, list_changed: bool) -> Self {
        self.capabilities.prompts = Some(PromptsCapability {
            list_changed: Some(list_changed),
        });
        self
    }

    /// Add a resource that will be available when the server starts.
    pub fn add_resource(mut self, resource: Resource) -> Self {
        self.resources.push(resource);
        self
    }

    /// Add a tool that will be available when the server starts.
    pub fn add_tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add a prompt that will be available when the server starts.
    pub fn add_prompt(mut self, prompt: Prompt) -> Self {
        self.prompts.push(prompt);
        self
    }

    /// Add experimental capabilities.
    pub fn with_experimental(mut self, experimental: impl Into<HashMap<String, serde_json::Value>>) -> Self {
        self.capabilities.experimental = Some(experimental.into());
        self
    }

    /// Add logging support.
    pub fn with_logging(mut self) -> Self {
        self.capabilities.logging = Some(HashMap::new());
        self
    }

    /// Build the server with the configured options.
    pub fn build(self) -> Result<Server> {
        let server = Server::new(self.capabilities, self.implementation);

        // Register any provided resources
        if let Some(handler) = server.resource_handler() {
            for resource in self.resources {
                handler.register_resource(resource)?;
            }
        } else if !self.resources.is_empty() {
            return Err(Error::InvalidRequest("resources support not enabled".into()));
        }

        // Register any provided tools
        if let Some(handler) = server.tool_handler() {
            for tool in self.tools {
                handler.register_tool(tool)?;
            }
        } else if !self.tools.is_empty() {
            return Err(Error::InvalidRequest("tools support not enabled".into()));
        }

        // Register any provided prompts
        if let Some(handler) = server.prompt_handler() {
            for prompt in self.prompts {
                handler.register_prompt(prompt)?;
            }
        } else if !self.prompts.is_empty() {
            return Err(Error::InvalidRequest("prompts support not enabled".into()));
        }

        Ok(server)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_schema::TextContent;

    #[test]
    fn test_builder_basic() {
        let server = ServerBuilder::new("test", "1.0")
            .with_resources(true, true)
            .with_tools(true)
            .with_prompts(true)
            .build()
            .unwrap();

        assert!(server.resource_handler().is_some());
        assert!(server.tool_handler().is_some());
        assert!(server.prompt_handler().is_some());
    }

    #[test]
    fn test_builder_with_items() {
        let resource = Resource {
            uri: "test://resource".into(),
            name: "Test Resource".into(),
            description: Some("A test resource".into()),
            mime_type: Some("text/plain".into()),
            annotated: Default::default(),
        };

        let tool = Tool {
            name: "test_tool".into(),
            description: Some("A test tool".into()),
            input_schema: mcp_schema::ToolInputSchema {
                type_: "object".into(),
                properties: None,
                required: None,
            },
            extra: Default::default(),
        };

        let prompt = Prompt {
            name: "test_prompt".into(),
            description: Some("A test prompt".into()),
            arguments: None,
            extra: Default::default(),
        };

        let server = ServerBuilder::new("test", "1.0")
            .with_resources(true, true)
            .with_tools(true)
            .with_prompts(true)
            .add_resource(resource)
            .add_tool(tool)
            .add_prompt(prompt)
            .build()
            .unwrap();

        assert!(server.resource_handler().is_some());
        assert!(server.tool_handler().is_some());
        assert!(server.prompt_handler().is_some());
    }

    #[test]
    fn test_builder_validation() {
        // Should fail when adding items without enabling support
        let resource = Resource {
            uri: "test://resource".into(),
            name: "Test Resource".into(),
            description: None,
            mime_type: None,
            annotated: Default::default(),
        };

        let result = ServerBuilder::new("test", "1.0")
            .add_resource(resource)
            .build();

        assert!(result.is_err());
    }
}
