mod builder;
mod context;
mod template;
#[cfg(test)]
mod tests;
mod tools;

pub use builder::PromptBuilder;
pub use context::{Message, PromptContext};
pub use template::PromptTemplate;
pub use tools::{Tool, ToolFunction};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Template error: {0}")]
    Template(String),

    #[error("Invalid tool: {0}")]
    InvalidTool(String),
}

pub type Result<T> = std::result::Result<T, Error>;
