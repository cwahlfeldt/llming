use hyperax::common::{Response, Body};
use hyperax::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct MockHyperax {
    responses: Arc<Mutex<Vec<Result<Response, Error>>>>,
}

impl MockHyperax {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(vec![])),
        }
    }

    pub async fn push_response(&self, response: Result<Response, Error>) {
        self.responses.lock().await.push(response);
    }

    pub async fn mock_success(&self, body: Vec<u8>) {
        let response = Response {
            status: 200,
            headers: vec![],
            body,
        };
        self.push_response(Ok(response)).await;
    }

    pub async fn mock_error(&self, status: u16) {
        let response = Response {
            status,
            headers: vec![],
            body: vec![],
        };
        self.push_response(Ok(response)).await;
    }
}