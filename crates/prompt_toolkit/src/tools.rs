use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    name: String,
    description: String,
    functions: Vec<ToolFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    name: String,
    description: String,
    parameters: String,
}

impl Tool {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            functions: Vec::new(),
        }
    }

    pub fn add_function(mut self, function: ToolFunction) -> Self {
        self.functions.push(function);
        self
    }

    pub fn format(&self) -> String {
        let mut output = String::with_capacity(
            self.name.len() + 
            self.description.len() + 
            self.functions.len() * 100
        );

        output.push_str("Tool: ");
        output.push_str(&self.name);
        output.push_str("\nDescription: ");
        output.push_str(&self.description);
        
        if !self.functions.is_empty() {
            output.push_str("\nFunctions:\n");
            for func in &self.functions {
                output.push_str(&func.format());
                output.push('\n');
            }
        }
        
        output
    }
}

impl ToolFunction {
    pub fn new(
        name: impl Into<String>, 
        description: impl Into<String>,
        parameters: impl Into<String>
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: parameters.into(),
        }
    }

    pub fn format(&self) -> String {
        let mut output = String::with_capacity(
            self.name.len() + 
            self.description.len() + 
            self.parameters.len() + 50
        );

        output.push_str("  - ");
        output.push_str(&self.name);
        output.push_str(": ");
        output.push_str(&self.description);
        output.push_str("\n    Parameters: ");
        output.push_str(&self.parameters);
        
        output
    }
}
