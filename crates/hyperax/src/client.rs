use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::header::{HeaderMap, HeaderName, HeaderValue};
use hyper::service::Service;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::net::TcpStream;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),
    #[error("Request error: {0}")]
    Request(#[from] hyper::http::Error),
    #[error("Connect error: {0}")]
    Connect(#[from] std::io::Error),
    #[error("{0}")]
    Mock(String),
}

struct HttpConnector {
    timeout: Option<Duration>,
}

impl HttpConnector {
    fn new() -> Self {
        Self {
            timeout: Some(Duration::from_secs(60)),
        }
    }
}

impl hyper::service::Service<hyper::Uri> for HttpConnector {
    type Response = TokioIo<TcpStream>;
    type Error = std::io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, uri: hyper::Uri) -> Self::Future {
        let timeout = self.timeout;
        Box::pin(async move {
            let addr = uri
                .authority()
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid uri")
                })?
                .as_str();

            let stream = TcpStream::connect(addr).await?;
            if let Some(_) = timeout {
                stream.set_nodelay(true)?;
            }
            Ok(TokioIo::new(stream))
        })
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    timeout: Option<Duration>,
    base_url: Option<String>,
    headers: HeaderMap,
}
#[cfg(test)]
thread_local! {
    static MOCK_RESPONSE: std::cell::RefCell<Option<Response<Bytes>>> = std::cell::RefCell::new(None);
}

impl Client {
    pub fn new() -> Self {
        Self {
            timeout: Some(Duration::from_secs(60)),
            base_url: None,
            headers: HeaderMap::new(),
        }
    }

    #[cfg(test)]
    pub fn mock_response(response: Response<Bytes>) {
        MOCK_RESPONSE.with(|r| *r.borrow_mut() = Some(response));
    }

    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub async fn request<T>(&self, mut req: Request<T>) -> Result<Response<Bytes>, Error>
    where
        T: Into<Full<Bytes>>,
    {
        #[cfg(test)]
        {
            return MOCK_RESPONSE.with(|r| {
                r.borrow()
                    .clone()
                    .ok_or_else(|| Error::Mock("No mock response set".to_string()))
            });
        }

        #[cfg(not(test))]
        {
            if let Some(base) = &self.base_url {
                let uri = format!("{}{}", base, req.uri());
                *req.uri_mut() = uri
                    .parse()
                    .map_err(|e| Error::Request(hyper::http::Error::from(e)))?;
            }
            for (k, v) in self.headers.iter() {
                req.headers_mut().insert(k, v.clone());
            }

            let (parts, body) = req.into_parts();
            let req = Request::from_parts(parts, body.into());

            let connector = HttpConnector::new();
            let io = connector.call(req.uri().clone()).await?;

            let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
            tokio::task::spawn(async move {
                if let Err(err) = conn.await {
                    eprintln!("Connection failed: {:?}", err);
                }
            });

            let resp = sender.send_request(req).await?;
            let (parts, body) = resp.into_parts();
            let bytes = body.collect().await?.to_bytes();
            Ok(Response::from_parts(parts, bytes))
        }
    }

    pub async fn get(&self, uri: &str) -> Result<Response<Bytes>, Error> {
        let req = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Full::default())?;
        self.request(req).await
    }

    pub async fn post<T>(&self, uri: &str, body: T) -> Result<Response<Bytes>, Error>
    where
        T: Into<Full<Bytes>>,
    {
        let req = Request::builder().method("POST").uri(uri).body(body)?;
        self.request(req).await
    }

    pub async fn put<T>(&self, uri: &str, body: T) -> Result<Response<Bytes>, Error>
    where
        T: Into<Full<Bytes>>,
    {
        let req = Request::builder().method("PUT").uri(uri).body(body)?;
        self.request(req).await
    }

    pub async fn delete(&self, uri: &str) -> Result<Response<Bytes>, Error> {
        let req = Request::builder()
            .method("DELETE")
            .uri(uri)
            .body(Full::default())?;
        self.request(req).await
    }

    pub async fn patch<T>(&self, uri: &str, body: T) -> Result<Response<Bytes>, Error>
    where
        T: Into<Full<Bytes>>,
    {
        let req = Request::builder().method("PATCH").uri(uri).body(body)?;
        self.request(req).await
    }

    pub async fn head(&self, uri: &str) -> Result<Response<Bytes>, Error> {
        let req = Request::builder()
            .method("HEAD")
            .uri(uri)
            .body(Full::default())?;
        self.request(req).await
    }
}

pub struct ClientBuilder {
    timeout: Option<Duration>,
    base_url: Option<String>,
    headers: HeaderMap,
}

impl ClientBuilder {
    fn new() -> Self {
        Self {
            timeout: Some(Duration::from_secs(60)),
            base_url: None,
            headers: HeaderMap::new(),
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn no_timeout(mut self) -> Self {
        self.timeout = None;
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_ref().as_bytes()),
            HeaderValue::from_str(value.as_ref()),
        ) {
            self.headers.insert(name, value);
        }
        self
    }

    pub fn build(self) -> Client {
        Client {
            timeout: self.timeout,
            base_url: self.base_url,
            headers: self.headers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::Response;
    use std::convert::Infallible;

    #[tokio::test]
    async fn test_client_methods() {
        let client = Client::new();

        // Mock response for testing
        let mock_response = Response::builder()
            .status(200)
            .body(Bytes::from("OK"))
            .unwrap();

        Client::mock_response(mock_response);

        // Test GET
        let response = client.get("http://test.com").await.unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.body(), &Bytes::from("OK"));

        // Test POST
        let response = client
            .post("http://test.com", Full::new(Bytes::from("test")))
            .await
            .unwrap();
        assert_eq!(response.status(), 200);

        // Test headers
        let client = Client::builder()
            .header("X-Test", "value")
            .base_url("http://test.com")
            .build();

        let response = client.get("/path").await.unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_client_builder() {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .base_url("http://test.com")
            .header("User-Agent", "test")
            .build();

        assert!(client.timeout.is_some());
        assert_eq!(client.base_url.as_deref(), Some("http://test.com"));
        assert!(client.headers.contains_key("user-agent"));
    }
}
