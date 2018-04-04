//! S3 service handles uploading static assets like images and videos to s3

pub mod client;
pub mod credentials;
pub mod error;
pub mod preprocessors;
pub mod random;
pub mod types;

use futures::future;
use futures::future::Future;
use futures_cpupool::CpuPool;
use rusoto_core::region::Region;
use rusoto_core::request::{HttpClient, TlsError};
use rusoto_s3::S3Client as CrateS3Client;
use services::types::ImageFormat;
use std::rc::Rc;
use tokio_core::reactor::Handle;

use self::client::S3Client;
use self::error::S3Error;
use self::preprocessors::{Image, ImageImpl};
use self::random::{Random, RandomImpl};
use self::types::ImageSize;

/// S3 service
#[derive(Clone)]
pub struct S3 {
    inner: Rc<S3Client>,
    bucket: String,
    cpu_pool: Rc<CpuPool>,
    random: Rc<Random>,
    image_preprocessor_factory: Rc<for<'a> Fn(&'a CpuPool) -> Box<Image + 'a>>,
}

impl S3 {
    /// Create s3 service
    ///
    /// * `bucket` - AWS s3 bucket name
    /// * `client` - client that implements S3Client trait
    /// * `image_preprocessor_factory` - closure that given a CPUPool reference returns Image
    pub fn new<F>(bucket: &str, client: Box<S3Client>, random: Box<Random>, image_preprocessor_factory: F) -> Self
    where
        F: for<'a> Fn(&'a CpuPool) -> Box<Image + 'a> + 'static,
    {
        // s3 doesn't require a region
        Self {
            inner: client.into(),
            bucket: bucket.to_string(),
            cpu_pool: Rc::new(CpuPool::new_num_cpus()),
            random: random.into(),
            image_preprocessor_factory: Rc::new(image_preprocessor_factory),
        }
    }

    /// Create s3 service
    ///
    /// * `key` - AWS key for s3 (from AWS console).
    /// * `secret` - AWS secret for s3 (from AWS console).
    /// * `bucket` - AWS s3 bucket name
    /// * `handle` - tokio event loop handle (needed for s3 http client)
    pub fn create(key: &str, secret: &str, bucket: &str, handle: &Handle) -> Result<Self, TlsError> {
        let credentials = credentials::Credentials::new(key.to_string(), secret.to_string());
        let client = HttpClient::new(handle)?;
        let random = RandomImpl::new();
        Ok(Self::new(
            bucket,
            Box::new(CrateS3Client::new(client, credentials, Region::UsEast1)),
            Box::new(random),
            |cpu_pool| Box::new(ImageImpl::new(cpu_pool)),
        ))
    }

    /// Uploads image along with all resized variants in `ImageSize` enum. If original image size is less
    /// than e.g. ImageSize::Large, then original image is uploaded instead of large.
    ///
    /// * `format` - now only "png" or "jpg" are supported
    /// * `bytes` - bytes repesenting compessed image (compessed with `image_type` codec)
    ///
    /// #Errors
    /// * `S3Error::Image` if encoding is incorrect, incl zero dimensions
    pub fn upload_image(&self, format: ImageFormat, bytes: Vec<u8>) -> Box<Future<Item = String, Error = S3Error>> {
        let random_hash = self.random.generate_hash();
        let original_name = Self::create_aws_name("img", "png", &ImageSize::Original, &random_hash);
        let url = format!("https://s3.amazonaws.com/{}/{}", self.bucket, original_name);
        let preprocessor = (*self.image_preprocessor_factory)(&*self.cpu_pool);
        let self_clone = self.clone();
        Box::new(preprocessor.process(format, bytes).and_then(move |images_hash| {
            let futures = images_hash
                .into_iter()
                .map(move |(size, bytes)| self_clone.upload_image_with_size(&random_hash, &size, bytes));
            future::join_all(futures).map(move |_| url)
        }))
    }

    /// Uploads an image with specific size to S3
    ///
    /// * `random_hash` - technically a filename for image
    /// * `size` - image size for deriving a name tag, like `dsf-small.png`
    /// * `image_type` - either "png", "jpg" or "jpeg" - these are types that are supported
    /// * `bytes` - bytes repesenting compessed image (compessed with `image_type` codec)
    fn upload_image_with_size(&self, random_hash: &str, size: &ImageSize, bytes: Vec<u8>) -> Box<Future<Item = (), Error = S3Error>> {
        let name = Self::create_aws_name("img", "png", size, random_hash);
        self.inner.upload(self.bucket.clone(), name, Some("image/png".to_string()), bytes)
    }

    fn create_aws_name(prefix: &str, image_type: &str, size: &ImageSize, random_hash: &str) -> String {
        let name = match size {
            &ImageSize::Original => format!("{}-{}.{}", prefix, random_hash, image_type), // don't use postfix if this is original image
            _ => format!("{}-{}-{}.{}", prefix, random_hash, size, image_type),
        };
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    struct RandomMock {
        hash: String,
    }

    impl RandomMock {
        fn new(hash: &str) -> Self {
            Self { hash: hash.to_string() }
        }
    }

    impl Random for RandomMock {
        fn generate_hash(&self) -> String {
            self.hash.clone()
        }
    }

    struct ImageMock<'a> {
        _cpu_pool: &'a CpuPool,
    }

    impl<'a> ImageMock<'a> {
        pub fn new(cpu_pool: &'a CpuPool) -> Self {
            Self { _cpu_pool: cpu_pool }
        }
    }

    impl<'a> Image for ImageMock<'a> {
        fn process(&self, _format: ImageFormat, _bytes: Vec<u8>) -> Box<Future<Item = HashMap<ImageSize, Vec<u8>>, Error = S3Error>> {
            let mut result = HashMap::new();
            result.insert(ImageSize::Thumb, b"thumb".to_vec());
            result.insert(ImageSize::Small, b"small".to_vec());
            result.insert(ImageSize::Medium, b"medium".to_vec());
            result.insert(ImageSize::Large, b"large".to_vec());
            result.insert(ImageSize::Original, b"original".to_vec());
            Box::new(future::ok(result))
        }
    }

    struct S3ClientMock {
        pub uploads: Rc<RefCell<HashMap<String, Vec<u8>>>>,
    }

    impl S3ClientMock {
        fn new() -> Self {
            S3ClientMock {
                uploads: Rc::new(RefCell::new(HashMap::new())),
            }
        }
    }

    impl S3Client for S3ClientMock {
        fn upload(
            &self,
            _bucket: String,
            key: String,
            _content_type: Option<String>,
            bytes: Vec<u8>,
        ) -> Box<Future<Item = (), Error = S3Error>> {
            let mut uploads = self.uploads.borrow_mut();
            uploads.insert(key, bytes);
            Box::new(future::ok(()))
        }
    }

    #[test]
    fn test_upload_image() {
        let random = RandomMock::new("somehash");
        let client = S3ClientMock::new();
        let uploads = client.uploads.clone();
        let s3 = S3::new("test-bucket", Box::new(client), Box::new(random), |cpu_pool| {
            Box::new(ImageMock::new(cpu_pool))
        });
        let mut expected_uploads = HashMap::new();
        expected_uploads.insert("img-somehash-thumb.png".to_string(), b"thumb".to_vec());
        expected_uploads.insert("img-somehash-small.png".to_string(), b"small".to_vec());
        expected_uploads.insert("img-somehash-medium.png".to_string(), b"medium".to_vec());
        expected_uploads.insert("img-somehash-large.png".to_string(), b"large".to_vec());
        expected_uploads.insert("img-somehash.png".to_string(), b"original".to_vec());

        let url = s3.upload_image(ImageFormat::PNG, b"".to_vec()).wait().unwrap();
        assert_eq!(url, "https://s3.amazonaws.com/test-bucket/img-somehash.png");
        assert_eq!(&*uploads.borrow(), &expected_uploads);
    }
}
