use crate::error::Result;
use crate::models::Model;
use async_trait::async_trait;
use std::sync::Arc;
use super::Handler;

#[allow(clippy::module_name_repetitions)]
pub struct McpHandler {
    model: Arc<dyn Model>,
}

impl McpHandler {
    #[must_use]
    pub fn new(model: Arc<dyn Model>) -> Self {
        Self { model }
    }
}