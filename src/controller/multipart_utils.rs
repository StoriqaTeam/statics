use std::io::{Error as StdError, Read};
use std::cmp;

use multipart::server::HttpRequest;
use hyper;
use hyper::header::ContentType;
use mime;

pub struct Bytes {
    pub inner: Vec<u8>,
}

impl Read for Bytes {
    // Todo - this method needs optimization, as its probably doing more copies than necessary
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, StdError> {
        let amt = cmp::min(buf.len(), self.inner.len());
        let bytes = self.inner.drain(..amt).collect::<Vec<u8>>();
        buf[..amt].copy_from_slice(&bytes[..]);

        Ok(amt)
    }
}

pub struct MultipartRequest {
    body: Bytes,
    headers: hyper::Headers,
    method: hyper::Method,
}

impl MultipartRequest {
    pub fn new(method: hyper::Method, headers: hyper::Headers, body: Vec<u8>) -> Self {
        Self {
            method,
            headers,
            body: Bytes { inner: body },
        }
    }
}

impl HttpRequest for MultipartRequest {
    type Body = Bytes;
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
    #[fail(display = "Failed to parse multipart")] Parse,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vecex_read() {
        let mut ex1 = VecEx {
            inner: vec![1, 2, 3, 4, 5],
        };
        let buf = &mut vec![0, 0];
        assert_eq!(ex1.read(buf).unwrap(), 2);
        assert_eq!(buf, &vec![1, 2]);
        assert_eq!(ex1.inner, vec![3u8, 4u8, 5u8]);

        assert_eq!(ex1.read(buf).unwrap(), 2);
        assert_eq!(buf, &vec![3, 4]);
        assert_eq!(ex1.inner, vec![5u8]);

        assert_eq!(ex1.read(buf).unwrap(), 1);
        assert_eq!(buf, &vec![5, 4]);
        assert_eq!(ex1.inner, vec![] as Vec<u8>);

        assert_eq!(ex1.read(buf).unwrap(), 0);
        assert_eq!(buf, &vec![5, 4]);
        assert_eq!(ex1.inner, vec![] as Vec<u8>);
    }
}
