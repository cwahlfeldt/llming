use bytes::Bytes;
use http_body_util::Full;
use hyperax::{Client, Server};
use std::convert::Infallible;
use std::net::SocketAddr;

#[tokio::test]
async fn test_basic_request() {
    // Start a test server
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = Server::new(addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Run server in background
    let server_handle = tokio::spawn(async move {
        server.run(|_req| async {
            Ok::<_, Infallible>(
                http::Response::builder()
                    .status(200)
                    .body(Full::new(Bytes::from("Hello, World!")))
                    .unwrap(),
            )
        })
        .await
    });

    // Test client request
    let client = Client::new();
    let uri = format!("http://{}", addr);
    let response = client.get(&uri).await.unwrap();
    
    assert_eq!(response.status(), 200);
    assert_eq!(response.body(), &Bytes::from("Hello, World!"));

    server_handle.abort();
}