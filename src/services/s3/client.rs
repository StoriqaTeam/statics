//! Client for AWS S3

use rusoto_core::request::HttpClient;
use rusoto_s3::{PutObjectRequest, S3, S3Client as CrateS3Client};
use futures::future::Future;

use super::error::S3Error;
use super::credentials::Credentials;

pub trait S3Client {
    /// Uploads raw bytes to s3 with filename `key` and content-type (used for serving file from s3)
    fn upload(&self, bucket: String, key: String, content_type: Option<String>, bytes: Vec<u8>) -> Box<Future<Item = (), Error = S3Error>>;
}

impl S3Client for CrateS3Client<Credentials, HttpClient> {
    fn upload(&self, bucket: String, key: String, content_type: Option<String>, bytes: Vec<u8>) -> Box<Future<Item = (), Error = S3Error>> {
        let request = PutObjectRequest {
            acl: Some("public-read".to_string()),
            body: Some(bytes),
            bucket,
            cache_control: None,
            content_disposition: None,
            content_encoding: None,
            content_language: None,
            content_length: None,
            content_md5: None,
            content_type,
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

        Box::new(self.put_object(&request).map(|_| ()).map_err(|e| e.into()))
    }
}
