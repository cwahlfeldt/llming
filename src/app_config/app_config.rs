use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ui: UIConfig,
    pub mcp_servers: Vec<MCPServerConfig>,
    pub llm: LLMConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    pub window_title: String,
    pub theme: String,
    pub default_width: u32,
    pub default_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    pub name: String,
    pub server_type: String,
    pub address: SocketAddr,
    pub allowed_paths: Vec<PathBuf>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub model: String,
    pub api_base_url: String,
    pub timeout_seconds: u64,
    pub system_prompt: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ui: UIConfig {
                window_title: "Chat App".to_string(),
                theme: "CatppuccinMocha".to_string(),
                default_width: 1200,
                default_height: 1000,
            },
            mcp_servers: vec![MCPServerConfig {
                name: "filesystem".to_string(),
                server_type: "filesystem".to_string(),
                address: "[::1]:3456".parse().unwrap(),
                allowed_paths: vec![
                    PathBuf::from("/home/waffles"),
                    PathBuf::from("/home/waffles/code"),
                ],
                enabled: true,
            }],
            llm: LLMConfig {
                model: "deepseek".to_string(),
                api_base_url: "https://api.deepseek.com/v1".to_string(),
                timeout_seconds: 30,
                system_prompt: include_str!("../prompts/system.txt").to_string(),
            },
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        // Try to load from config file
        if let Ok(config_str) = std::fs::read_to_string("config.toml") {
            if let Ok(config) = toml::from_str(&config_str) {
                return config;
            }
        }

        // Fall back to default if loading fails
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_str = toml::to_string_pretty(self)?;
        std::fs::write("config.toml", config_str)?;
        Ok(())
    }
}
