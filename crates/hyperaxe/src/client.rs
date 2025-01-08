use anyhow::Result;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request};
use hyper_tls::HttpsConnector;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use tracing::{debug, error};

#[derive(Clone, Debug)]
pub struct HttpClient {
    client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
    headers: HashMap<String, String>,
}

impl Default for HttpClient {
    fn default() -> Self {
        let https = HttpsConnector::new();
        Self {
            client: Client::builder(TokioExecutor::new()).build(https),
            headers: HashMap::new(),
        }
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub async fn send_request<T, R>(&self, method: Method, url: &str, body: Option<T>) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        debug!("Sending {} request to {}", method, url);
        let mut builder = Request::builder().method(method).uri(url);

        // Add headers
        for (key, value) in &self.headers {
            debug!("Adding header: {} = {}", key, value);
            builder = builder.header(key, value);
        }

        // Add body if present
        let req = if let Some(body) = body {
            let body_str = serde_json::to_string(&body)?;
            debug!("Request body: {}", body_str);
            builder
                .header("content-type", "application/json")
                .body(Full::new(Bytes::from(body_str)))?
        } else {
            builder.body(Full::new(Bytes::from("")))?
        };

        // Send request
        debug!("Sending request");
        let response = match self.client.request(req).await {
            Ok(resp) => resp,
            Err(e) => {
                error!("Failed to send request: {}", e);
                return Err(anyhow::anyhow!("Failed to send request: {}", e));
            }
        };

        // Check status
        let status = response.status();
        debug!("Response status: {}", status);

        if !status.is_success() {
            let body_bytes = response.collect().await?.to_bytes();
            let error_body = String::from_utf8_lossy(&body_bytes);
            error!("Request failed: {} - {}", status, error_body);
            return Err(anyhow::anyhow!(
                "Request failed with status {}: {}",
                status,
                error_body
            ));
        }

        // Parse response
        let body_bytes = response.collect().await?.to_bytes();
        let body_str = String::from_utf8_lossy(&body_bytes);
        debug!("Response body: {}", body_str);

        match serde_json::from_str(&body_str) {
            Ok(response_data) => {
                debug!("Successfully parsed response");
                Ok(response_data)
            }
            Err(e) => {
                error!("Failed to parse response: {}", e);
                Err(anyhow::anyhow!("Failed to parse response: {}", e))
            }
        }
    }
}
