use async_trait::async_trait;
use mcp_schema::{JSONRPCNotification, ListRootsResult, Root, ServerResult, JSONRPC_VERSION};
use serde_json::Value;
use std::sync::{Arc, RwLock};

use crate::handler::RequestHandler;
use crate::{Error, Result};

/// State management for root URIs
#[derive(Debug, Default)]
pub(crate) struct RootState {
    roots: Vec<Root>,
}

/// Handler for root-related requests
pub struct RootHandler {
    state: Arc<RwLock<RootState>>,
    notify: Option<Arc<Box<dyn Fn(JSONRPCNotification<Value>) + Send + Sync>>>,
}

impl RootHandler {
    /// Create a new root handler
    pub fn new(notify: Option<Arc<Box<dyn Fn(JSONRPCNotification<Value>) + Send + Sync>>>) -> Self {
        Self {
            state: Arc::new(RwLock::new(RootState::default())),
            notify,
        }
    }

    /// Add a root URI to the list
    pub fn add_root(&self, root: Root) -> Result<()> {
        let mut state = self
            .state
            .write()
            .map_err(|_| Error::Internal("state lock poisoned".into()))?;

        // Validate root URI scheme (must be file:// for now)
        if !root.uri.starts_with("file://") {
            return Err(Error::InvalidRequest(
                "root URI must start with file://".into(),
            ));
        }

        state.roots.push(root);
        self.notify_change();
        Ok(())
    }

    /// Remove a root URI from the list
    pub fn remove_root(&self, uri: &str) -> Result<()> {
        let mut state = self
            .state
            .write()
            .map_err(|_| Error::Internal("state lock poisoned".into()))?;

        state.roots.retain(|r| r.uri != uri);
        self.notify_change();
        Ok(())
    }

    /// Clear all roots
    pub fn clear_roots(&self) -> Result<()> {
        let mut state = self
            .state
            .write()
            .map_err(|_| Error::Internal("state lock poisoned".into()))?;

        state.roots.clear();
        self.notify_change();
        Ok(())
    }

    /// Get a list of all roots
    pub fn list_roots(&self) -> Result<Vec<Root>> {
        let state = self
            .state
            .read()
            .map_err(|_| Error::Internal("state lock poisoned".into()))?;

        Ok(state.roots.clone())
    }

    /// Send a roots/list_changed notification
    fn notify_change(&self) {
        if let Some(notify) = &self.notify {
            let notification = JSONRPCNotification {
                json_rpc: JSONRPC_VERSION.to_string(),
                method: "notifications/roots/list_changed".to_string(),
                params: Value::Object(Default::default()),
            };
            (notify)(notification);
        }
    }
}

#[async_trait]
impl RequestHandler for RootHandler {
    async fn handle(&self, _params: Value) -> Result<ServerResult> {
        // Handle roots/list request
        let roots = self.list_roots()?;

        Ok(ServerResult::ListRoots(ListRootsResult {
            meta: None,
            roots,
            extra: Default::default(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn test_root() -> Root {
        Root {
            uri: "file:///test/path".into(),
            name: Some("Test Root".into()),
            extra: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_root_management() {
        let notifications = Arc::new(Mutex::new(Vec::new()));
        let notif_clone = notifications.clone();

        let handler = RootHandler::new(Some(Arc::new(Box::new(move |n| {
            notif_clone.lock().unwrap().push(n);
        }))));

        // Test adding roots
        handler.add_root(test_root()).unwrap();

        let roots = handler.list_roots().unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].uri, "file:///test/path");

        // Test removing roots
        handler.remove_root("file:///test/path").unwrap();
        assert!(handler.list_roots().unwrap().is_empty());

        // Verify notifications were sent
        let notifications = notifications.lock().unwrap();
        assert_eq!(notifications.len(), 2); // One for add, one for remove
        assert_eq!(notifications[0].method, "notifications/roots/list_changed");
    }

    #[test]
    fn test_invalid_root() {
        let handler = RootHandler::new(None);

        // Test adding invalid root URI
        let result = handler.add_root(Root {
            uri: "http://invalid".into(),
            name: None,
            extra: Default::default(),
        });
        assert!(result.is_err());
    }
}
