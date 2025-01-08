use crate::protocol::common::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelledNotification {
    pub request_id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressNotification {
    pub progress_token: ProgressToken,
    pub progress: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}