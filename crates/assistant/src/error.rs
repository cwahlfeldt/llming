use thiserror::Error;

#[derive(Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum AssistantError {
    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Invalid configuration: {0}")]
    Config(String),

    #[error("Model error: {0}")]
    Model(String),
}

impl From<hyperax::Error> for AssistantError {
    fn from(err: hyperax::Error) -> Self {
        match err {
            hyperax::Error::Http(e) => AssistantError::Network(format!("HTTP error: {e}")),
            hyperax::Error::Request(e) => AssistantError::Network(format!("Request error: {e}")),
            hyperax::Error::Connect(e) => AssistantError::Network(format!("Connection error: {e}")),
        }
    }
}

pub type Result<T> = std::result::Result<T, AssistantError>;