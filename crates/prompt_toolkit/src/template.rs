use crate::{PromptContext, Tool};

#[derive(Debug, Clone)]
pub struct PromptTemplate {
    prefix: Option<String>,
    suffix: Option<String>,
    tools: Vec<Tool>,
}

impl PromptTemplate {
    pub fn new() -> Self {
        Self {
            prefix: None,
            suffix: None,
            tools: Vec::new(),
        }
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn add_tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn add_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools.extend(tools);
        self
    }

    pub fn format(&self, context: &PromptContext) -> String {
        let mut capacity = context.format().len();

        if let Some(prefix) = &self.prefix {
            capacity += prefix.len();
        }

        if let Some(suffix) = &self.suffix {
            capacity += suffix.len();
        }

        if !self.tools.is_empty() {
            // Estimate tool formatting capacity
            capacity += self.tools.len() * 100;
        }

        let mut output = String::with_capacity(capacity);

        if let Some(prefix) = &self.prefix {
            output.push_str(prefix);
            output.push('\n');
        }

        if !self.tools.is_empty() {
            output.push_str("Available tools:\n");
            for tool in &self.tools {
                output.push_str(&tool.format());
                output.push('\n');
            }
        }

        output.push_str(&context.format());

        if let Some(suffix) = &self.suffix {
            output.push('\n');
            output.push_str(suffix);
        }

        output
    }
}
