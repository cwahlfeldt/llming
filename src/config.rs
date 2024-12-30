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
