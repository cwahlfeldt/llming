use crate::{PromptTemplate, Result, Tool};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default)]
pub struct PromptBuilder {
    tools: Vec<Tool>,
    template_text: Option<String>,
    variables: HashMap<String, Value>,
}

impl PromptBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_template(mut self, template: &str) -> Self {
        self.template_text = Some(template.to_string());
        self
    }

    pub fn with_variable<T: serde::Serialize>(mut self, key: &str, value: T) -> Self {
        self.variables
            .insert(key.to_string(), serde_json::to_value(value).unwrap());
        self
    }

    pub fn build(self) -> Result<PromptTemplate> {
        let template_text = self
            .template_text
            .ok_or_else(|| crate::Error::Template("Template text must be provided".to_string()))?;

        let prompt_template = PromptTemplate::new()
            .with_prefix(template_text)
            .with_suffix("")
            .add_tools(self.tools);

        Ok(prompt_template)
    }
}
