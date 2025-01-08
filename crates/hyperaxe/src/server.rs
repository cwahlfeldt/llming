use anyhow::Result;
use bytes::Bytes;
use http_body_util::Full;
use hyper::service::service_fn;
use hyper::{body::Incoming, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde::Serialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};

#[derive(Clone, Debug)]
pub struct HttpServer {
    addr: SocketAddr,
}

impl HttpServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub async fn serve<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Request<Incoming>) -> Fut + Clone + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Response<Full<Bytes>>>> + Send,
    {
        let listener = TcpListener::bind(self.addr).await?;
        info!("HTTP Server listening on {}", self.addr);

        loop {
            match listener.accept().await {
                Ok((tcp_stream, client_addr)) => {
                    debug!("Accepted connection from: {}", client_addr);
                    let io = TokioIo::new(tcp_stream);
                    let handler = handler.clone();

                    tokio::spawn(async move {
                        let service = service_fn(move |req| {
                            let handler = handler.clone();
                            debug!("Handling request: {} {}", req.method(), req.uri());
                            async move {
                                match handler(req).await {
                                    Ok(response) => {
                                        debug!("Handler succeeded: {}", response.status());
                                        Ok::<_, anyhow::Error>(response)
                                    }
                                    Err(e) => {
                                        error!("Handler error: {}", e);
                                        let error_response = Response::builder()
                                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                                            .header("content-type", "application/json")
                                            .body(Full::new(Bytes::from(format!(
                                                "{{\"error\": \"{}\"}}",
                                                e
                                            ))))?;
                                        Ok(error_response)
                                    }
                                }
                            }
                        });

                        if let Err(err) = hyper::server::conn::http1::Builder::new()
                            .serve_connection(io, service)
                            .await
                        {
                            error!("Connection error: {:?}", err);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                    // Add a small delay before retrying
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }

    pub async fn json_response<T: Serialize>(data: T) -> Result<Response<Full<Bytes>>> {
        let json = serde_json::to_string(&data)?;
        debug!("Sending JSON response: {}", json);
        Ok(Response::builder()
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(json)))?)
    }

    pub async fn error_response(
        status: StatusCode,
        message: &str,
    ) -> Result<Response<Full<Bytes>>> {
        warn!("Sending error response: {} - {}", status, message);
        Ok(Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(format!(
                "{{\"error\": \"{}\"}}",
                message
            ))))?)
    }
}
