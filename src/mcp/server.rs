use anyhow::Result;
use bytes::{Buf, Bytes};
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::client::{
    MCPFunction, MCPFunctionCall, MCPFunctionResult, MCPPrompt, MCPServerInfo, MCPTool,
};

type AsyncFunctionHandler = Arc<
    dyn Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;
type PromptRenderer = Arc<dyn Fn(serde_json::Value) -> Result<String> + Send + Sync>;

pub struct MCPServer {
    info: MCPServerInfo,
    functions: Arc<RwLock<HashMap<String, AsyncFunctionHandler>>>,
    prompt_renderers: Arc<RwLock<HashMap<String, PromptRenderer>>>,
    http_server: crate::http::HttpServer,
}

impl std::fmt::Debug for MCPServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MCPServer")
            .field("info", &self.info)
            .field("http_server", &self.http_server)
            .finish()
    }
}

impl MCPServer {
    pub fn new(addr: std::net::SocketAddr, info: MCPServerInfo) -> Self {
        Self {
            info,
            functions: Arc::new(RwLock::new(HashMap::new())),
            prompt_renderers: Arc::new(RwLock::new(HashMap::new())),
            http_server: crate::http::HttpServer::new(addr),
        }
    }

    pub fn get_info(&self) -> &MCPServerInfo {
        &self.info
    }

    pub async fn register_async_function<F, Fut>(
        &self,
        tool_name: &str,
        function_name: &str,
        handler: F,
    ) where
        F: Fn(serde_json::Value) -> Pin<Box<Fut>> + Send + Sync + 'static,
        Fut: Future<Output = Result<serde_json::Value>> + Send + 'static,
    {
        let mut functions = self.functions.write().await;
        functions.insert(
            format!("{}.{}", tool_name, function_name),
            Arc::new(move |params| Box::pin(handler(params).into_future())),
        );
    }

    pub async fn register_prompt_renderer<F>(&self, prompt_name: &str, renderer: F)
    where
        F: Fn(serde_json::Value) -> Result<String> + Send + Sync + 'static,
    {
        let mut renderers = self.prompt_renderers.write().await;
        renderers.insert(prompt_name.to_string(), Arc::new(renderer));
    }

    async fn handle_request(&self, req: Request<Incoming>) -> Result<Response<Full<Bytes>>> {
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/mcp/info") => crate::http::HttpServer::json_response(&self.info).await,

            (&Method::POST, "/mcp/function") => {
                let body = req.into_body();
                let collected = body.collect().await?;
                let bytes = collected.aggregate();
                let call: MCPFunctionCall = serde_json::from_slice(bytes.chunk())?;

                let functions = self.functions.read().await;
                if let Some(handler) = functions.get(&call.function) {
                    let result = handler(call.parameters).await?;
                    crate::http::HttpServer::json_response(MCPFunctionResult { result }).await
                } else {
                    crate::http::HttpServer::error_response(
                        StatusCode::NOT_FOUND,
                        "Function not found",
                    )
                    .await
                }
            }

            (&Method::GET, path) if path.starts_with("/mcp/prompt/") => {
                let prompt_name = path.trim_start_matches("/mcp/prompt/");
                if let Some(prompt) = self.info.prompts.iter().find(|p| p.name == prompt_name) {
                    crate::http::HttpServer::json_response(prompt).await
                } else {
                    crate::http::HttpServer::error_response(
                        StatusCode::NOT_FOUND,
                        "Prompt not found",
                    )
                    .await
                }
            }

            (&Method::POST, path)
                if path.starts_with("/mcp/prompt/") && path.ends_with("/render") =>
            {
                let prompt_name = path
                    .trim_start_matches("/mcp/prompt/")
                    .trim_end_matches("/render")
                    .to_string();

                let body = req.into_body();
                let collected = body.collect().await?;
                let bytes = collected.aggregate();
                let params: serde_json::Value = serde_json::from_slice(bytes.chunk())?;

                let renderers = self.prompt_renderers.read().await;
                if let Some(renderer) = renderers.get(&prompt_name) {
                    let rendered = renderer(params)?;
                    crate::http::HttpServer::json_response(serde_json::json!({
                        "rendered": rendered
                    }))
                    .await
                } else {
                    crate::http::HttpServer::error_response(
                        StatusCode::NOT_FOUND,
                        "Prompt renderer not found",
                    )
                    .await
                }
            }

            _ => crate::http::HttpServer::error_response(StatusCode::NOT_FOUND, "Not found").await,
        }
    }

    pub async fn serve(&self) -> Result<()> {
        let server = Arc::new(self.clone());
        self.http_server
            .serve(move |req| {
                let server = server.clone();
                async move { server.handle_request(req).await }
            })
            .await
    }
}

impl Clone for MCPServer {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
            functions: self.functions.clone(),
            prompt_renderers: self.prompt_renderers.clone(),
            http_server: self.http_server.clone(),
        }
    }
}
