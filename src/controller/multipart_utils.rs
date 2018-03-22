use multipart::server::HttpRequest;
use hyper;
use hyper::header::ContentType;
use mime;
use utils::Bytes;

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
            body: Bytes::new(body),
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
