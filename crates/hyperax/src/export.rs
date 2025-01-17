//! Re-exports of commonly used types from hyper and related crates

pub use hyper::{
    header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Request, Response, StatusCode, Method,
    body::Bytes,
};
pub use http_body_util::BodyExt;

pub use crate::client::{Client, Error as ClientError};
