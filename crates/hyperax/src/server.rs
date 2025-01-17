use http_body_util::Full;
use hyper::body::Bytes;
use hyper::service::Service;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::watch;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Server {
    addr: SocketAddr,
}

impl Server {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub async fn run<F, Fut>(
        &mut self,
        handler: F,
        mut shutdown: watch::Receiver<bool>,
    ) -> Result<(), Error>
    where
        F: Fn(Request<hyper::body::Incoming>) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<Response<Full<Bytes>>, Infallible>> + Send + 'static,
    {
        let listener = TcpListener::bind(self.addr).await?;
        let active_connections = Arc::new(AtomicUsize::new(0));

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, _)) => {
                            let io = TokioIo::new(stream);
                            let handler = handler.clone();
                            let mut shutdown = shutdown.clone();
                            let connections = active_connections.clone();
                            connections.fetch_add(1, Ordering::SeqCst);

                            tokio::task::spawn(async move {
                                let conn = hyper::server::conn::http1::Builder::new()
                                    .serve_connection(io, HandlerService(handler));

                                // Run connection until shutdown signal received
                                tokio::select! {
                                    result = conn => {
                                        if let Err(err) = result {
                                            eprintln!("Error serving connection: {:?}", err);
                                        }
                                    }
                                    _ = shutdown.changed() => {}
                                }

                                connections.fetch_sub(1, Ordering::SeqCst);
                            });
                        }
                        Err(e) => eprintln!("Accept error: {}", e),
                    }
                }
                _ = shutdown.changed() => {
                    println!("Server shutting down...");
                    // Wait for active connections to complete
                    while active_connections.load(Ordering::SeqCst) > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                    return Ok(());
                }
            }
        }
    }
}

struct HandlerService<F>(F);

impl<F, Fut> Service<Request<hyper::body::Incoming>> for HandlerService<F>
where
    F: Fn(Request<hyper::body::Incoming>) -> Fut,
    Fut: Future<Output = Result<Response<Full<Bytes>>, Infallible>>,
{
    type Response = Response<Full<Bytes>>;
    type Error = Infallible;
    type Future = Fut;

    fn call(&self, req: Request<hyper::body::Incoming>) -> Self::Future {
        (self.0)(req)
    }
}

// async fn main() {
//     let mut server = Server::new(SocketAddr::from(([127, 0, 0, 1], 8080)));
//     let (tx, rx) = watch::channel(false);
//     server
//         .run(
//             |_req| async {
//                 Ok::<_, Infallible>(
//                     Response::builder()
//                         .status(200)
//                         .body(Full::new(Bytes::from("Hello World")))
//                         .unwrap(),
//                 )
//             },
//             rx,
//         )
//         .await
//         .unwrap();
// }

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http_body_util::Full;
    use std::convert::Infallible;
    use std::net::SocketAddr;
    use tokio::sync::watch;

    #[tokio::test]
    async fn test_server_basic() {
        // Setup
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let mut server = Server::new(addr);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Track request count
        let request_count = Arc::new(AtomicUsize::new(0));
        let request_count_clone = request_count.clone();

        // Start server with mock handler
        let server_handle = tokio::spawn(async move {
            server
                .run(
                    move |_req| {
                        request_count_clone.fetch_add(1, Ordering::SeqCst);
                        async move {
                            Ok::<_, Infallible>(
                                Response::builder()
                                    .status(200)
                                    .body(Full::new(Bytes::from("mock response")))
                                    .unwrap(),
                            )
                        }
                    },
                    shutdown_rx,
                )
                .await
        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Trigger shutdown
        shutdown_tx
            .send(true)
            .expect("Failed to send shutdown signal");

        // Verify server shuts down
        let result =
            tokio::time::timeout(tokio::time::Duration::from_millis(100), server_handle).await;

        assert!(result.is_ok(), "Server failed to shut down in time");
        assert_eq!(
            request_count.load(Ordering::SeqCst),
            0,
            "No requests should have been processed"
        );
    }
}
