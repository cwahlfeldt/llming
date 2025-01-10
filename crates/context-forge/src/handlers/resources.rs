use async_trait::async_trait;
use mcp_schema::{
    ListResourceTemplatesResult, ListResourcesResult, ReadResourceResult, Resource,
    ResourceTemplate, ServerResult,
};
use serde_json::Value;
use std::collections::HashMap;

use super::{HandlerState, SharedState};
use crate::handler::RequestHandler;
use crate::{Error, Result};

/// State for the resources handler
#[derive(Default)]
pub(crate) struct ResourceState {
    resources: HashMap<String, Resource>,
    templates: HashMap<String, ResourceTemplate>,
    subscriptions: HashMap<String, Vec<String>>, // uri -> subscriber_ids
}

impl HandlerState for ResourceState {}

/// Handler for resource-related requests
#[derive(Clone)]
pub struct ResourceHandler {
    state: SharedState<ResourceState>,
}

impl Default for ResourceHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceHandler {
    pub fn new() -> Self {
        Self {
            state: SharedState::default(),
        }
    }

    /// Register a resource that can be accessed by clients
    pub fn register_resource(&self, resource: Resource) -> Result<()> {
        let mut state = self
            .state
            .write()
            .map_err(|_| Error::Internal("state lock poisoned".into()))?;
        state.resources.insert(resource.uri.clone(), resource);
        Ok(())
    }

    /// Register a resource template
    pub fn register_template(&self, template: ResourceTemplate) -> Result<()> {
        let mut state = self
            .state
            .write()
            .map_err(|_| Error::Internal("state lock poisoned".into()))?;
        state
            .templates
            .insert(template.uri_template.clone(), template);
        Ok(())
    }
}

#[async_trait]
impl RequestHandler for ResourceHandler {
    async fn handle(&self, params: Value) -> Result<ServerResult> {
        // Get the method from request params
        let method = params
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::InvalidParams("missing method".into()))?;

        match method {
            "resources/list" => {
                let state = self
                    .state
                    .read()
                    .map_err(|_| Error::Internal("state lock poisoned".into()))?;
                let resources = state.resources.values().cloned().collect();
                Ok(ServerResult::ListResources(ListResourcesResult {
                    meta: None,
                    next_cursor: None,
                    resources,
                    extra: Default::default(),
                }))
            }
            "resources/templates/list" => {
                let state = self
                    .state
                    .read()
                    .map_err(|_| Error::Internal("state lock poisoned".into()))?;
                let resource_templates = state.templates.values().cloned().collect();
                Ok(ServerResult::ListResourceTemplates(
                    ListResourceTemplatesResult {
                        meta: None,
                        next_cursor: None,
                        resource_templates,
                        extra: Default::default(),
                    },
                ))
            }
            "resources/read" => {
                let uri = params
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::InvalidParams("missing uri".into()))?;

                let state = self
                    .state
                    .read()
                    .map_err(|_| Error::Internal("state lock poisoned".into()))?;

                let resource = state
                    .resources
                    .get(uri)
                    .ok_or_else(|| Error::InvalidRequest(format!("resource not found: {}", uri)))?;

                // At this point we'd actually read the resource content
                // For now just return empty result
                Ok(ServerResult::ReadResource(ReadResourceResult {
                    meta: None,
                    contents: Vec::new(),
                    extra: Default::default(),
                }))
            }
            "resources/subscribe" => {
                // Handle subscription
                Ok(ServerResult::Empty(mcp_schema::EmptyResult {
                    meta: None,
                    extra: Default::default(),
                }))
            }
            "resources/unsubscribe" => {
                // Handle unsubscribe
                Ok(ServerResult::Empty(mcp_schema::EmptyResult {
                    meta: None,
                    extra: Default::default(),
                }))
            }
            _ => Err(Error::MethodNotFound(method.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_resource() -> Resource {
        Resource {
            uri: "test://resource".into(),
            name: "Test Resource".into(),
            description: Some("A test resource".into()),
            mime_type: Some("text/plain".into()),
            annotated: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_resource_list() {
        let handler = ResourceHandler::new();
        handler.register_resource(test_resource()).unwrap();

        let result = handler
            .handle(json!({
                "method": "resources/list",
            }))
            .await
            .unwrap();

        match result {
            ServerResult::ListResources(result) => {
                assert_eq!(result.resources.len(), 1);
                assert_eq!(result.resources[0].uri, "test://resource");
            }
            _ => panic!("Expected ListResources result"),
        }
    }
}
