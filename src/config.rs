
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub window_pos: Option<(i32, i32)>,
    pub window_size: Option<(u32, u32)>,
    pub anthropic: AnthropicConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            model: "claude-3.5-sonnet".to_string(),
            max_tokens: 1024,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_pos: None,
            window_size: Some((800, 600)),
            anthropic: AnthropicConfig::default(),
        }
    }
}
