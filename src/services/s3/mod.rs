// rusoto_core::request::HttpClient
pub mod credentials;

use tokio_core::reactor::Handle;
use rusoto_core::request::{HttpClient, TlsError};
use rusoto_core::region::Region;
use rusoto_core::RusotoFuture;
use rusoto_s3::{S3 as S3Trait, S3Client, PutObjectOutput, PutObjectError, PutObjectRequest};

pub struct S3 {
    inner: S3Client<credentials::Credentials, HttpClient>,
}

impl S3 {
    pub fn new(key: String, secret: String, handle: &Handle) -> Result<Self, TlsError> {
        let credentials = credentials::Credentials::new(key, secret);
        let client = HttpClient::new(handle)?;
        // s3 doesn't require a region
        Ok(Self { inner: S3Client::new(client, credentials, Region::UsEast1) })
    }

    pub fn upload(&self, bytes: Vec<u8>) -> RusotoFuture<PutObjectOutput, PutObjectError> {
        self.raw_upload("storiqa-dev".to_string(), "test".to_string(), bytes)
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
