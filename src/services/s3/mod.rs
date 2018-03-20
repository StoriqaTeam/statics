pub mod credentials;

use rand;
use rand::Rng;
use base64::encode;

use futures::future::Future;
use tokio_core::reactor::Handle;
use rusoto_core::request::{HttpClient, TlsError};
use rusoto_core::region::Region;
use rusoto_core::RusotoFuture;
use rusoto_s3::{S3 as S3Trait, S3Client, PutObjectOutput, PutObjectError, PutObjectRequest};

pub struct S3 {
    inner: S3Client<credentials::Credentials, HttpClient>,
}

static HASH_LEN_BYTES: u8 = 8;

impl S3 {
    pub fn new(key: String, secret: String, handle: &Handle) -> Result<Self, TlsError> {
        let credentials = credentials::Credentials::new(key, secret);
        let client = HttpClient::new(handle)?;
        // s3 doesn't require a region
        Ok(Self { inner: S3Client::new(client, credentials, Region::UsEast1) })
    }

    pub fn upload(&self, image_type: &str, bytes: Vec<u8>) -> Box<Future<Item=String, Error=PutObjectError>> {
        let bytes = Vec::with_capacity(HASH_LEN_BYTES as usize);
        let buffer = &mut bytes[..];
        rand::thread_rng().fill_bytes(buffer);
        let name = format!("{}.{}", encode(buffer), image_type);
        Box::new(
            self.raw_upload("storiqa-dev".to_string(), name.to_string(), bytes).map(|_| name.to_string())
        )
    }

    pub fn raw_upload(&self, bucket: String, key: String, bytes: Vec<u8>) -> RusotoFuture<PutObjectOutput, PutObjectError> {
        let request = PutObjectRequest {
            acl: None,
            body: Some(bytes),
            bucket,
            cache_control: None,
            content_disposition: None,
            content_encoding: None,
            content_language: None,
            content_length: None,
            content_md5: None,
            content_type: None,
            expires: None,
            grant_full_control: None,
            grant_read: None,
            grant_read_acp: None,
            grant_write_acp: None,
            key,
            metadata: None,
            request_payer: None,
            sse_customer_algorithm: None,
            sse_customer_key: None,
            sse_customer_key_md5: None,
            ssekms_key_id: None,
            server_side_encryption: None,
            storage_class: None,
            tagging: None,
            website_redirect_location: None,
        };
        self.inner.put_object(&request)
    }
}
