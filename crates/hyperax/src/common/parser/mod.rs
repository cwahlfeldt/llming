use bytes::{Buf, Bytes};
use http::{Request, Response, HeaderMap, Version};
use std::io::{self, Cursor};

pub struct Parser {
    buf: Cursor<Bytes>,
}

impl Parser {
    pub fn new(bytes: Bytes) -> Self {
        Self {
            buf: Cursor::new(bytes),
        }
    }

    pub fn parse_request(&mut self) -> io::Result<Request<()>> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        
        let status = req.parse(self.buf.chunk())?;
        
        if !status.is_complete() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "incomplete HTTP request",
            ));
        }

        let mut builder = Request::builder()
            .method(req.method.unwrap())
            .uri(req.path.unwrap())
            .version(Version::HTTP_11);

        if let Some(headers) = builder.headers_mut() {
            for h in req.headers {
                headers.insert(
                    h.name.parse().unwrap(),
                    h.value.to_owned().into(),
                );
            }
        }

        Ok(builder.body(()).unwrap())
    }

    pub fn parse_response(&mut self) -> io::Result<Response<()>> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut res = httparse::Response::new(&mut headers);
        
        let status = res.parse(self.buf.chunk())?;
        
        if !status.is_complete() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "incomplete HTTP response",
            ));
        }

        let mut builder = Response::builder()
            .status(res.code.unwrap())
            .version(Version::HTTP_11);

        if let Some(headers) = builder.headers_mut() {
            for h in res.headers {
                headers.insert(
                    h.name.parse().unwrap(),
                    h.value.to_owned().into(),
                );
            }
        }

        Ok(builder.body(()).unwrap())
    }
}