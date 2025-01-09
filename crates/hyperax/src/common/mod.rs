use http_body_util::Full;
use hyper::body::Bytes;

pub struct Body(Vec<u8>);

impl Body {
    pub fn new(data: impl Into<Vec<u8>>) -> Self {
        Self(data.into())
    }

    pub fn empty() -> Self {
        Self(Vec::new())
    }
}

impl From<Body> for Full<Bytes> {
    fn from(body: Body) -> Self {
        Full::new(Bytes::from(body.0))
    }
}

#[derive(Clone, Debug)]
pub struct Header {
    name: String,
    value: String,
}

impl Header {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
}
