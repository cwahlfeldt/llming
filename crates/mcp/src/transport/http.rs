use crate::protocol::messages::JSONRPCMessage;
use crate::{Error, Result};
use async_trait::async_trait;
use futures::Stream;
use hyperaxe::client::{Client, ClientBuilder};
use hyperaxe::http::{HeaderMap, HeaderValue};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

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
        let response = self
            .client
            .post(&self.endpoint)
            .headers(headers)
            .json(&message)?
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Transport(
                format!("HTTP request failed with status: {}", response.status()).into(),
            ));
        }

        Ok(())
    }

    async fn receive(&self) -> Result<JSONRPCMessage> {
        // HTTP transport doesn't support receiving messages directly
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
    client_config: Option<ClientBuilder>,
}

impl HttpTransportBuilder {
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Result<Self> {
        self.headers.insert(
            key,
            HeaderValue::from_str(value).map_err(|e| Error::Transport(e.into()))?,
        );
        Ok(self)
    }

    pub fn client_config(mut self, config: ClientBuilder) -> Self {
        self.client_config = Some(config);
        self
    }

    pub fn build(self) -> Result<HttpTransport> {
        let endpoint = self
            .endpoint
            .ok_or_else(|| Error::Transport("HTTP transport requires an endpoint".into()))?;

        let client = self.client_config.unwrap_or_default().build()?;

        Ok(HttpTransport {
            client,
            endpoint,
            headers: Arc::new(RwLock::new(self.headers)),
        })
    }
}
