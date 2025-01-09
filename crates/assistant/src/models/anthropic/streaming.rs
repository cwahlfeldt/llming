use crate::error::Result;
use futures::Stream;
use std::pin::Pin;

pub type StreamResponse = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

pub struct StreamOptions {
    pub chunk_size: usize,
    pub timeout: std::time::Duration,
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            chunk_size: 1024,
            timeout: std::time::Duration::from_secs(30),
        }
    }
}

pub trait Streamable {
    fn stream(&self, message: &str, options: StreamOptions) -> StreamResponse;
}