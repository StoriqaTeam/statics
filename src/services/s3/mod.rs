pub mod credentials;
pub mod error;

use std::sync::Arc;
use std::fmt::{Display, Error, Formatter};
use std::collections::HashMap;
use rand;
use rand::Rng;
use base64::encode;
use futures::future;
use futures::future::Future;
use tokio_core::reactor::Handle;
use rusoto_core::request::{HttpClient, TlsError};
use rusoto_core::region::Region;
use rusoto_s3::{PutObjectError, PutObjectOutput, PutObjectRequest, S3 as S3Trait, S3Client};
use image;
use image::GenericImage;
use futures_cpupool::CpuPool;
use futures::sync::oneshot;
use image::DynamicImage;

use self::error::S3Error;

#[derive(PartialEq, Eq, Hash, Clone)]
enum Size {
    Thumb = 40,
    Small = 80,
    Medium = 320,
    Large = 640,
}

impl Display for Size {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            &Size::Thumb => f.write_str("thumb"),
            &Size::Small => f.write_str("small"),
            &Size::Medium => f.write_str("medium"),
            &Size::Large => f.write_str("large"),
        }
    }
}

#[derive(Clone)]
pub struct S3 {
    inner: Arc<S3Client<credentials::Credentials, HttpClient>>,
    bucket: String,
    cpu_pool: Arc<CpuPool>,
}

static HASH_LEN_BYTES: u8 = 8;

impl S3 {
    pub fn new(key: &str, secret: &str, bucket: &str, handle: &Handle) -> Result<Self, TlsError> {
        let credentials = credentials::Credentials::new(key.to_string(), secret.to_string());
        let client = HttpClient::new(handle)?;
        // s3 doesn't require a region
        Ok(Self {
            inner: Arc::new(S3Client::new(client, credentials, Region::UsEast1)),
            bucket: bucket.to_string(),
            cpu_pool: Arc::new(CpuPool::new_num_cpus()),
        })
    }

    pub fn upload_image(&self, image_type: &str, bytes: Vec<u8>) -> Box<Future<Item = String, Error = S3Error>> {
        let content_type = format!("image/{}", image_type);
        let random_hash = Self::generate_random_hash();
        let name = Self::create_aws_name("img", image_type, None, &random_hash);
        let url = format!("https://s3.amazonaws.com/{}/{}", self.bucket, name);
        let image_format = match &image_type[..] {
            "png" => image::ImageFormat::PNG,
            "jpg" | "jpeg" => image::ImageFormat::JPEG,
            _ => {
                return S3Error::Image(format!(
                    "Unsupported image type: {}",
                    image_type
                )).into()
            }
        };
        let image = match image::load_from_memory_with_format(&bytes, image_format) {
            Ok(data) => data,
            Err(e) => return S3Error::Image(format!("Error paring image format: {}", e)).into()
        };

        let mut futures: Vec<_> = vec![Size::Thumb, Size::Small, Size::Medium, Size::Large].iter().map(|size| {
            let img = image.clone();
            self.resize_and_upload_image_async(size, &content_type, image_type, &random_hash, img)
        }).collect();
        futures.push(self.raw_upload(name, Some(content_type), bytes));
        Box::new(future::join_all(futures).map(move |_| url))
    }

    fn resize_and_upload_image_async(
        &self,
        size: &Size,
        content_type: &str,
        image_type: &str,
        random_hash: &str,
        image: DynamicImage,
    ) -> Box<Future<Item = (), Error = S3Error>> {
        let self_clone = self.clone();
        let size_clone = size.clone();
        let content_type_clone = content_type.to_string();
        let image_type_clone = image_type.to_string();
        let random_hash_clone = random_hash.to_string();

        Box::new(
            self.resize_image_async(size, content_type, image_type, random_hash, image)
                .and_then(move |bytes| {
                    let name = Self::create_aws_name("img", &image_type_clone, Some(&size_clone), &random_hash_clone);
                    self_clone.raw_upload(name, Some(content_type_clone), bytes)
                })
        )
    }

    fn resize_image_async(
        &self,
        size: &Size,
        content_type: &str,
        image_type: &str,
        random_hash: &str,
        image: DynamicImage,
    ) -> Box<Future<Item = Vec<u8>, Error = S3Error>> {
        let size = size.clone();
        let content_type = content_type.to_string();
        let image_type = image_type.to_string();
        let random_hash = random_hash.to_string();
        Box::new(
            self.cpu_pool.spawn_fn(move || -> Result<Vec<u8>, S3Error> {
                let name = Self::create_aws_name("img", &image_type, Some(&size), &random_hash);
                let (w, h) = image.dimensions();
                let smallest_dimension = if w < h { w } else { h };
                if smallest_dimension == 0 {
                    return Err(S3Error::Image("Uploaded image size is zero".to_string()));
                }
                let int_size = size.clone() as u32;
                let width = ((w as f32) * (int_size as f32) / (smallest_dimension as f32)).round() as u32;
                let height = ((h as f32) * (int_size as f32) / (smallest_dimension as f32)).round() as u32;
                let resized_image = match int_size {
                    x if x < smallest_dimension => image.resize_exact(width, height, image::FilterType::Triangle),
                    _ => image,
                };
                let mut buffer = Vec::new();
                let _ = resized_image.save(&mut buffer, image::ImageFormat::PNG);
                Ok(buffer)
            })
        )
    }

    pub fn raw_upload(
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

    fn generate_random_hash() -> String {
        let mut name_bytes = vec![0; HASH_LEN_BYTES as usize];
        let buffer = name_bytes.as_mut_slice();
        rand::thread_rng().fill_bytes(buffer);
        Self::encode_for_aws(&encode(buffer))
    }

    fn create_aws_name(prefix: &str, image_type: &str, size: Option<&Size>, random_hash: &str) -> String {
        let name = match size {
            Some(size) => format!("{}-{}-{}.{}", prefix, random_hash, size, image_type),
            None => format!("{}-{}.{}", prefix, random_hash, image_type),
        };
        name
    }

    fn encode_for_aws(s: &str) -> String {
        let s = s.replace("+", "A");
        let s = s.replace("/", "B");
        s.replace("=", "C")
    }
}
