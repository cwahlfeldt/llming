use crate::protocol::messages::JSONRPCMessage;
use crate::{Error, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use http::header::{HeaderMap, HeaderValue};
use http_body_util::Full;
use hyperax::Client;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct HttpTransport {
    client: Client,
    endpoint: String,
    headers: Arc<RwLock<HeaderMap>>,
}

impl HttpTransport {
    pub fn builder() -> HttpTransportBuilder {
        HttpTransportBuilder::default()
    }
}

#[async_trait]
impl super::Transport for HttpTransport {
    async fn send(&self, message: JSONRPCMessage) -> Result<()> {
        let headers = self.headers.read().await.clone();
        let body = serde_json::to_vec(&message)?;

        let mut req = http::Request::builder().method("POST").uri(&self.endpoint);

        for (key, value) in headers.iter() {
            req = req.header(key, value);
        }

        let req = req
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body)))
            .map_err(|e| Error::Transport(e.to_string().into()))?;

        let response = self
            .client
            .request(req)
            .await
            .map_err(|e| Error::Transport(e.to_string().into()))?;

        if !response.status().is_success() {
            return Err(Error::Transport(
                format!("HTTP request failed with status: {}", response.status()).into(),
            ));
        }

        Ok(())
    }

    async fn receive(&self) -> Result<JSONRPCMessage> {
        Err(Error::Transport(
            "HTTP transport doesn't support receiving messages directly".into(),
        ))
    }

    fn message_stream(&self) -> Option<Pin<Box<dyn Stream<Item = Result<JSONRPCMessage>> + Send>>> {
        None
    }
}

#[derive(Default)]
pub struct HttpTransportBuilder {
    endpoint: Option<String>,
    headers: HeaderMap,
}

impl HttpTransportBuilder {
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn header(mut self, key: impl Into<&'static str>, value: &str) -> Result<Self> {
        self.headers.insert(
            key.into(),
            HeaderValue::from_str(value).map_err(|e| Error::Transport(e.to_string().into()))?,
        );
        Ok(self)
    }

    pub fn build(self) -> Result<HttpTransport> {
        let endpoint = self
            .endpoint
            .ok_or_else(|| Error::Transport("HTTP transport requires an endpoint".into()))?;

        Ok(HttpTransport {
            client: Client::new(),
            endpoint,
            headers: Arc::new(RwLock::new(self.headers)),
        })
    }
}
