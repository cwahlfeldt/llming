use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRequest {
    pub ref_: CompletionReference,
    pub argument: CompletionArgument,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionArgument {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CompletionReference {
    Resource(ResourceReference),
    Prompt(PromptReference),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReference {
    #[serde(rename = "type")]
    pub ref_type: String, // "ref/resource"
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptReference {
    #[serde(rename = "type")]
    pub ref_type: String, // "ref/prompt"
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteResult {
    pub completion: CompletionOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    pub values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}