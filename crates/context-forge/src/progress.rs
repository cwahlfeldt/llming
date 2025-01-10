use mcp_schema::{JSONRPCNotification, ProgressNotificationParams, ProgressToken, JSONRPC_VERSION};
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// A progress tracker for long-running operations.
pub struct ProgressTracker {
    token: ProgressToken,
    progress: Arc<Mutex<f64>>,
    total: Option<f64>,
    notify: Box<dyn Fn(JSONRPCNotification<Value>) + Send + Sync>,
}

impl ProgressTracker {
    /// Create a new progress tracker with the given token and optional total.
    pub fn new(
        token: ProgressToken,
        total: Option<f64>,
        notify: impl Fn(JSONRPCNotification<Value>) + Send + Sync + 'static,
    ) -> Self {
        Self {
            token,
            progress: Arc::new(Mutex::new(0.0)),
            total,
            notify: Box::new(notify),
        }
    }

    /// Update the current progress and send a notification.
    pub fn update(&self, progress: f64) {
        let mut current = self.progress.lock().unwrap();
        *current = progress;

        // Create progress notification
        let notification = JSONRPCNotification {
            json_rpc: JSONRPC_VERSION.to_string(),
            method: "notifications/progress".to_string(),
            params: Value::from(
                serde_json::to_value(ProgressNotificationParams {
                    progress_token: self.token.clone(),
                    progress,
                    total: self.total,
                    extra: Default::default(),
                })
                .unwrap(),
            ),
        };

        // Send notification through callback
        (self.notify)(notification);
    }

    /// Get the current progress.
    pub fn current(&self) -> f64 {
        *self.progress.lock().unwrap()
    }

    /// Get the total if one was set.
    pub fn total(&self) -> Option<f64> {
        self.total
    }
}

/// Helper function to check if a request includes a progress token
pub fn get_progress_token(params: &Value) -> Option<ProgressToken> {
    params
        .get("_meta")?
        .as_object()?
        .get("progressToken")
        .and_then(|t| serde_json::from_value(t.clone()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_progress_tracking() {
        let notifications = Arc::new(Mutex::new(Vec::new()));
        let notif_clone = notifications.clone();

        let tracker = ProgressTracker::new(ProgressToken::Number(1), Some(100.0), move |n| {
            notif_clone.lock().unwrap().push(n)
        });

        // Test progress updates
        tracker.update(25.0);
        tracker.update(50.0);
        tracker.update(75.0);

        let notifs = notifications.lock().unwrap();
        assert_eq!(notifs.len(), 3);

        // Check last notification
        if let Value::Object(params) = &notifs[2].params {
            assert_eq!(params["progress"].as_f64().unwrap(), 75.0);
            assert_eq!(params["total"].as_f64().unwrap(), 100.0);
        } else {
            panic!("Expected object params");
        }
    }

    #[test]
    fn test_progress_token_extraction() {
        let params = serde_json::json!({
            "_meta": {
                "progressToken": 123
            }
        });

        let token = get_progress_token(&params);
        assert!(matches!(token, Some(ProgressToken::Number(123))));

        let params = serde_json::json!({
            "_meta": {
                "progressToken": "abc"
            }
        });

        let token = get_progress_token(&params);
        assert!(matches!(token, Some(ProgressToken::String(s)) if s == "abc"));
    }
}
