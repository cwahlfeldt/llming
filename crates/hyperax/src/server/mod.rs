use http_body_util::Full;
use hyper::body::Bytes;
use hyper::service::Service;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use tokio::net::TcpListener;

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

    pub async fn run<F, Fut>(&self, handler: F) -> Result<(), Error>
    where
        F: Fn(Request<hyper::body::Incoming>) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<Response<Full<Bytes>>, Infallible>> + Send + 'static,
    {
        let listener = TcpListener::bind(self.addr).await?;

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let handler = handler.clone();

            tokio::task::spawn(async move {
                if let Err(err) = hyper::server::conn::http1::Builder::new()
                    .serve_connection(io, HandlerService(handler))
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
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
