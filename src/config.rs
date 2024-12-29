use cosmic::cosmic_config::{self, ConfigGet, ConfigSet};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub window_pos: Option<(i32, i32)>,
    pub window_size: Option<(u32, u32)>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_pos: None,
            window_size: Some((800, 600)),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub history_path: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: String::from("gpt-3.5-turbo"),
            max_tokens: 2048,
            temperature: 0.7,
            history_path: dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("llming")
                .join("history"),
        }
    }
}
impl AppConfig {
    pub async fn load_or_default() -> anyhow::Result<Self> {
        let schema_path = "/com/waffles/llming/app/";
        let config = cosmic_config::Config::new(schema_path, 0)?;

        if let Ok(app_config) = Self::from_config(&config) {
            Ok(app_config)
        } else {
            let default_config = Self::default();
            default_config.write_config(&config);
            Ok(default_config)
        }
    }

    pub fn from_config(config: &cosmic_config::Config) -> anyhow::Result<Self> {
        Ok(config.get("")?)
    }

    pub async fn write_config(&self, config: &cosmic_config::Config) -> anyhow::Result<()> {
        Ok(config.set("", self)?)
    }
}
