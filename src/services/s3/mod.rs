//! S3 service handles uploading static assets like images and videos to s3

pub mod error;
pub mod credentials;
pub mod preprocessors;
pub mod types;

use std::sync::Arc;
use rand;
use rand::Rng;
use base64::encode;
use futures::future;
use futures::future::Future;
use tokio_core::reactor::Handle;
use rusoto_core::request::{HttpClient, TlsError};
use rusoto_core::region::Region;
use rusoto_s3::{PutObjectRequest, S3 as S3Trait, S3Client};
use futures_cpupool::CpuPool;

use self::preprocessors::{Image};
use self::error::S3Error;
use self::types::ImageSize;

/// Lenght of the random hash in s3 filename in bytes
static HASH_LEN_BYTES: u8 = 8;

/// S3 service
#[derive(Clone)]
pub struct S3 {
    inner: Arc<S3Client<credentials::Credentials, HttpClient>>,
    bucket: String,
    cpu_pool: Arc<CpuPool>,
    image_preprocessor_factory: Arc<Box<for<'a> Fn(&'a CpuPool) -> Box<Image + 'a>>>
}

impl S3 {
    /// Create s3 service
    ///
    /// * `key` - AWS key for s3 (from AWS console).
    /// * `secret` - AWS secret for s3 (from AWS console).
    /// * `bucket` - AWS s3 bucket name
    /// * `handle` - tokio event loop handle (needed for s3 http client)
    /// * `image_preprocessor_factory` - closure that given a CPUPool reference returns Image
    pub fn new<F>(key: &str, secret: &str, bucket: &str, handle: &Handle, image_preprocessor_factory: F ) -> Result<Self, TlsError>
    where F: for<'a> Fn(&'a CpuPool) -> Box<Image + 'a> + 'static
    {
        let credentials = credentials::Credentials::new(key.to_string(), secret.to_string());
        let client = HttpClient::new(handle)?;
        // s3 doesn't require a region
        Ok(Self {
            inner: Arc::new(S3Client::new(client, credentials, Region::UsEast1)),
            bucket: bucket.to_string(),
            cpu_pool: Arc::new(CpuPool::new_num_cpus()),
            image_preprocessor_factory: Arc::new(Box::new(image_preprocessor_factory)),
        })
    }

    /// Uploads image along with all resized variants in `ImageSize` enum. If original image size is less
    /// than e.g. ImageSize::Large, then original image is uploaded instead of large.
    ///
    /// * `image_type` - either "png", "jpg" or "jpeg" - these are types that are supported
    /// * `bytes` - bytes repesenting compessed image (compessed with `image_type` codec)
    pub fn upload_image(&self, image_type: &str, bytes: Vec<u8>) -> Box<Future<Item = String, Error = S3Error>> {
        let random_hash = Self::generate_random_hash();
        let original_name = Self::create_aws_name("img", "png", &ImageSize::Original, &random_hash);
        let url = format!("https://s3.amazonaws.com/{}/{}", self.bucket, original_name);
        let preprocessor = (*self.image_preprocessor_factory)(&*self.cpu_pool);
        let self_clone = self.clone();
        Box::new(
            preprocessor.process(image_type, bytes)
                .and_then(move |images_hash| {
                    let futures = images_hash.into_iter().map(move |(size, bytes)| self_clone.upload_image_with_size(&random_hash, &size, bytes));
                    future::join_all(futures).map(move |_| url)
                })
        )

    }

    /// Uploads an image with specific size to S3
    ///
    /// * `random_hash` - technically a filename for image
    /// * `size` - image size for deriving a name tag, like `dsf-small.png`
    /// * `image_type` - either "png", "jpg" or "jpeg" - these are types that are supported
    /// * `bytes` - bytes repesenting compessed image (compessed with `image_type` codec)
    fn upload_image_with_size(&self, random_hash: &str, size: &ImageSize, bytes: Vec<u8>) -> Box<Future<Item = (), Error = S3Error>> {
        let name = Self::create_aws_name("img", "png", size, random_hash);
        self.upload_data(name, Some("image/png".to_string()), bytes)
    }

    /// Uploads raw bytes to s3 with filename `key` and content-type (used for serving file from s3)
    fn upload_data(
        &self,
        key: String,
        content_type: Option<String>,
        bytes: Vec<u8>,
    ) -> Box<Future<Item = (), Error = S3Error>> {
        let request = PutObjectRequest {
            acl: Some("public-read".to_string()),
            body: Some(bytes),
            bucket: self.bucket.clone(),
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
        Box::new(self.inner.put_object(&request).map(|_| ()).map_err(|e| e.into()))
    }

    fn create_aws_name(prefix: &str, image_type: &str, size: &ImageSize, random_hash: &str) -> String {
        let name = match size {
            &ImageSize::Original => format!("{}-{}.{}", prefix, random_hash, image_type), // don't use postfix if this is original image
            _ => format!("{}-{}-{}.{}", prefix, random_hash, size, image_type),
        };
        name
    }

    fn generate_random_hash() -> String {
        let mut name_bytes = vec![0; HASH_LEN_BYTES as usize];
        let buffer = name_bytes.as_mut_slice();
        rand::thread_rng().fill_bytes(buffer);
        Self::encode_for_aws(&encode(buffer))
    }

    /// Three symbols +, /, = are not aws and url-friendly, just replace them
    fn encode_for_aws(s: &str) -> String {
        let s = s.replace("+", "A");
        let s = s.replace("/", "B");
        s.replace("=", "C")
    }
}
