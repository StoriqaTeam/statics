//! This module implements HttpRequest trait (part of `multipart` crate)
//! for hyper::Request. It might come as a surprise that we need to do this,
//! but at this point of time `multipart` crate was designed for `hyper` <= 0.10
//! which is synchronous and cannot be used with async hyper (> 0.11). There is
//! an async implementation https://github.com/abonander/multipart-async, but
//! it's in pre-alpha version.

use multipart::server::HttpRequest;
use hyper;
use hyper::header::ContentType;
use mime;
use std::io::Cursor;

/// Structure that complies with `multipart` crate HttpRequest
pub struct MultipartRequest {
    body: Cursor<Vec<u8>>,
    headers: hyper::Headers,
    method: hyper::Method,
}

impl MultipartRequest {
    pub fn new(method: hyper::Method, headers: hyper::Headers, body: Vec<u8>) -> Self {
        Self {
            method,
            headers,
            body: Cursor::new(body),
        }
    }
}

impl HttpRequest for MultipartRequest {
    type Body = Cursor<Vec<u8>>;
    fn multipart_boundary(&self) -> Option<&str> {
        if self.method != hyper::Method::Post {
            return None;
        }

        self.headers.get::<ContentType>().and_then(|ct| {
            let ContentType(ref mime) = *ct;
            let params = match (mime.type_(), mime.subtype(), mime.params()) {
                (mime::MULTIPART, mime::FORM_DATA, params) => params,
                _ => return None,
            };

            params
                .filter(|kv| kv.0 == mime::BOUNDARY)
                .next()
                .map(|kv| kv.1.as_str())
        })
    }
    fn body(self) -> Self::Body {
        self.body
    }
}

#[derive(Debug, Fail)]
pub enum MultipartError {
    #[fail(display = "Failed to parse multipart body")] Parse,
}
