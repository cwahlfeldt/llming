//! Error types for the context-forge library.

use mcp_schema::{PARSE_ERROR, INVALID_REQUEST, METHOD_NOT_FOUND, INVALID_PARAMS, INTERNAL_ERROR};
use thiserror::Error;

/// A specialized Result type for context-forge operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents errors that can occur in context-forge operations.
#[derive(Error, Debug)]
pub enum Error {
    /// The server is not initialized.
    #[error("server not initialized")]
    NotInitialized,

    /// The server is already initialized.
    #[error("server already initialized")]
    AlreadyInitialized,

    /// The received request was malformed or invalid.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// The requested method was not found.
    #[error("method not found: {0}")]
    MethodNotFound(String),

    /// The provided parameters were invalid.
    #[error("invalid parameters: {0}")]
    InvalidParams(String),

    /// An internal error occurred.
    #[error("internal error: {0}")]
    Internal(String),

    /// Error during JSON serialization/deserialization.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    /// Convert the error into a JSON-RPC error code and message.
    pub fn to_rpc_error(&self) -> (i32, String) {
        match self {
            Error::NotInitialized | Error::AlreadyInitialized => {
                (INVALID_REQUEST, self.to_string())
            }
            Error::InvalidRequest(_) => (INVALID_REQUEST, self.to_string()),
            Error::MethodNotFound(_) => (METHOD_NOT_FOUND, self.to_string()),
            Error::InvalidParams(_) => (INVALID_PARAMS, self.to_string()),
            Error::Internal(_) => (INTERNAL_ERROR, self.to_string()),
            Error::Json(_) => (PARSE_ERROR, self.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_to_rpc_error() {
        let error = Error::MethodNotFound("test.method".into());
        let (code, message) = error.to_rpc_error();
        assert_eq!(code, METHOD_NOT_FOUND);
        assert_eq!(message, "method not found: test.method");

        let error = Error::InvalidParams("missing field".into());
        let (code, message) = error.to_rpc_error();
        assert_eq!(code, INVALID_PARAMS);
        assert_eq!(message, "invalid parameters: missing field");
    }

    #[test]
    fn test_error_display() {
        let error = Error::NotInitialized;
        assert_eq!(error.to_string(), "server not initialized");

        let error = Error::Internal("test error".into());
        assert_eq!(error.to_string(), "internal error: test error");
    }
}
