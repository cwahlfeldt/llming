use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid JSON-RPC request: {0}")]
    InvalidRequest(String),
    
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Transport error: {0}")]
    Transport(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;