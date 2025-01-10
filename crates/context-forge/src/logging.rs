use async_trait::async_trait;
use mcp_schema::{
    JSONRPCNotification, LoggingLevel, LoggingMessageParams, ServerResult, SetLevelParams,
    JSONRPC_VERSION,
};
use serde_json::Value;
use std::sync::{Arc, RwLock};

use crate::handler::RequestHandler;
use crate::{Error, Result};

/// State for the logging system
#[derive(Clone)]
struct LogState {
    level: LoggingLevel,
    notify: Arc<Box<dyn Fn(JSONRPCNotification<Value>) + Send + Sync>>,
}

/// Handler for logging-related requests
pub struct LoggingHandler {
    state: Arc<RwLock<LogState>>,
}

impl LoggingHandler {
    /// Create a new logging handler with default level
    pub fn new(notify: impl Fn(JSONRPCNotification<Value>) + Send + Sync + 'static) -> Self {
        Self {
            state: Arc::new(RwLock::new(LogState {
                level: LoggingLevel::Info, // Default level
                notify: Arc::new(Box::new(notify)),
            })),
        }
    }

    /// Set the logging level
    pub fn set_level(&self, level: LoggingLevel) -> Result<()> {
        let mut state = self
            .state
            .write()
            .map_err(|_| Error::Internal("state lock poisoned".into()))?;
        state.level = level;
        Ok(())
    }

    /// Get current logging level
    pub fn get_level(&self) -> Result<LoggingLevel> {
        let state = self
            .state
            .read()
            .map_err(|_| Error::Internal("state lock poisoned".into()))?;
        Ok(state.level)
    }

    /// Log a message at the specified level
    pub fn log(&self, level: LoggingLevel, logger: Option<String>, data: Value) -> Result<()> {
        let state = self
            .state
            .read()
            .map_err(|_| Error::Internal("state lock poisoned".into()))?;

        // Check if this message should be logged based on level
        if !should_log(state.level, level) {
            return Ok(());
        }

        // Create and send notification
        let notification = JSONRPCNotification {
            json_rpc: JSONRPC_VERSION.to_string(),
            method: "notifications/message".to_string(),
            params: Value::from(
                serde_json::to_value(LoggingMessageParams {
                    level,
                    logger,
                    data,
                    extra: Default::default(),
                })
                .unwrap(),
            ),
        };

        (state.notify)(notification);
        Ok(())
    }
}

/// Helper to check if a message should be logged
fn should_log(current: LoggingLevel, msg_level: LoggingLevel) -> bool {
    // Order matters: emergency is highest severity, debug is lowest
    let levels = [
        LoggingLevel::Emergency,
        LoggingLevel::Alert,
        LoggingLevel::Critical,
        LoggingLevel::Error,
        LoggingLevel::Warning,
        LoggingLevel::Notice,
        LoggingLevel::Info,
        LoggingLevel::Debug,
    ];

    // Get indices of levels
    let current_idx = levels.iter().position(|l| l == &current).unwrap();
    let msg_idx = levels.iter().position(|l| l == &msg_level).unwrap();

    // Message level should be same or higher severity (lower index)
    msg_idx <= current_idx
}

#[async_trait]
impl RequestHandler for LoggingHandler {
    async fn handle(&self, params: Value) -> Result<ServerResult> {
        let params: SetLevelParams =
            serde_json::from_value(params).map_err(|e| Error::InvalidParams(e.to_string()))?;

        self.set_level(params.level)?;

        Ok(ServerResult::Empty(mcp_schema::EmptyResult {
            meta: None,
            extra: Default::default(),
        }))
    }
}

/// Convenience macros for logging
#[macro_export]
macro_rules! log {
    ($handler:expr, $level:expr, $($arg:tt)+) => {
        $handler.log(
            $level,
            Some(module_path!().to_string()),
            serde_json::json!({ "message": format!($($arg)+) })
        )
    };
}

#[macro_export]
macro_rules! debug {
    ($handler:expr, $($arg:tt)+) => {
        $crate::log!($handler, LoggingLevel::Debug, $($arg)+)
    };
}

#[macro_export]
macro_rules! info {
    ($handler:expr, $($arg:tt)+) => {
        $crate::log!($handler, LoggingLevel::Info, $($arg)+)
    };
}

#[macro_export]
macro_rules! warn {
    ($handler:expr, $($arg:tt)+) => {
        $crate::log!($handler, LoggingLevel::Warning, $($arg)+)
    };
}

#[macro_export]
macro_rules! error {
    ($handler:expr, $($arg:tt)+) => {
        $crate::log!($handler, LoggingLevel::Error, $($arg)+)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_should_log() {
        // Debug level logs everything
        assert!(should_log(LoggingLevel::Debug, LoggingLevel::Debug));
        assert!(should_log(LoggingLevel::Debug, LoggingLevel::Info));
        assert!(should_log(LoggingLevel::Debug, LoggingLevel::Warning));
        assert!(should_log(LoggingLevel::Debug, LoggingLevel::Error));

        // Info level doesn't log debug
        assert!(!should_log(LoggingLevel::Info, LoggingLevel::Debug));
        assert!(should_log(LoggingLevel::Info, LoggingLevel::Info));
        assert!(should_log(LoggingLevel::Info, LoggingLevel::Warning));
        assert!(should_log(LoggingLevel::Info, LoggingLevel::Error));

        // Error level only logs error and above
        assert!(!should_log(LoggingLevel::Error, LoggingLevel::Debug));
        assert!(!should_log(LoggingLevel::Error, LoggingLevel::Info));
        assert!(!should_log(LoggingLevel::Error, LoggingLevel::Warning));
        assert!(should_log(LoggingLevel::Error, LoggingLevel::Error));
    }

    #[tokio::test]
    async fn test_logging_handler() {
        let notifications = Arc::new(Mutex::new(Vec::new()));
        let notif_clone = notifications.clone();

        let handler = LoggingHandler::new(move |n| {
            notif_clone.lock().unwrap().push(n);
        });

        // Test setting level
        let params = serde_json::json!({
            "level": "warning"
        });
        handler.handle(params).await.unwrap();
        assert_eq!(handler.get_level().unwrap(), LoggingLevel::Warning);

        // Test logging at different levels
        handler
            .log(
                LoggingLevel::Error,
                Some("test".to_string()),
                serde_json::json!("error message"),
            )
            .unwrap();

        handler
            .log(
                LoggingLevel::Info,
                Some("test".to_string()),
                serde_json::json!("info message"),
            )
            .unwrap();

        // Only error message should be logged due to warning level
        let notifications = notifications.lock().unwrap();
        assert_eq!(notifications.len(), 1);

        if let Value::Object(params) = &notifications[0].params {
            assert_eq!(params["level"].as_str().unwrap(), "error");
            assert_eq!(params["logger"].as_str().unwrap(), "test");
        } else {
            panic!("Expected object params");
        }
    }

    #[test]
    fn test_log_macros() {
        let notifications = Arc::new(Mutex::new(Vec::new()));
        let notif_clone = notifications.clone();

        let handler = LoggingHandler::new(move |n| {
            notif_clone.lock().unwrap().push(n);
        });

        info!(handler, "test info message").unwrap();
        error!(handler, "test error: {}", "details").unwrap();

        let notifications = notifications.lock().unwrap();
        assert_eq!(notifications.len(), 2);
    }
}
