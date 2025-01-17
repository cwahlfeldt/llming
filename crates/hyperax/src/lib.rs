pub mod client;
pub mod common;
pub mod server;
mod export;

pub use client::{Client, Error};
pub use server::Server;
pub use export::*;

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use hyper::Response;

    #[tokio::test]
    async fn test_client_mock() {
        let client = Client::new();

        // Set up mock response
        let mock_response = Response::builder()
            .status(200)
            .body(Bytes::from("test response"))
            .unwrap();

        Client::mock_response(mock_response);

        // Test the client with mock
        let response = client.get("http://test.com").await.unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.body(), &Bytes::from("test response"));
    }
}
