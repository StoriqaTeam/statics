pub mod credentials;

use std::sync::Arc;
use std::fmt::{Display, Formatter, Error};
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
}

static HASH_LEN_BYTES: u8 = 8;

impl S3 {
    pub fn new(key: String, secret: String, handle: &Handle) -> Result<Self, TlsError> {
        let credentials = credentials::Credentials::new(key, secret);
        let client = HttpClient::new(handle)?;
        // s3 doesn't require a region
        Ok(Self {
            inner: Arc::new(S3Client::new(client, credentials, Region::UsEast1)),
        })
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

    fn upload_image_with_size(&self, size: Option<&Size>, bucket: &str, content_type: &str, image_type: &str, random_hash: &str, bytes: Vec<u8>) -> Box<Future<Item = PutObjectOutput, Error = PutObjectError>> {
        let name = Self::create_aws_name("img", image_type, size, random_hash);
        self.raw_upload(bucket.to_string(), name, Some(content_type.to_string()), bytes)
    }

    pub fn upload_image(&self, image_type: &str, bytes: Vec<u8>) -> Box<Future<Item = String, Error = PutObjectError>> {
        let content_type = format!("image/{}", image_type);
        let bucket = "storiqa-dev";
        let random_hash = Self::generate_random_hash();
        let name = Self::create_aws_name("img", image_type, None, &random_hash);
        let url = format!(
            "https://s3.amazonaws.com/{}/{}",
            bucket, name
        );
        if let Ok(image_hash) = Self::prepare_image(image_type, &bytes[..]) {
            let mut futures: Vec<_> = image_hash.keys().map(|size| {
                let bytes = image_hash.get(size).unwrap().to_vec();
                self.upload_image_with_size(Some(size), bucket, &content_type, image_type, &random_hash, bytes)
            }).collect();
            futures.push(self.upload_image_with_size(None, bucket, &content_type, image_type, &random_hash, bytes));
            Box::new(future::join_all(futures).map(move |_| url))
        } else {
            Box::new(future::err(PutObjectError::Unknown("failed to set image sizes".to_string()))) as Box<Future<Item = String, Error = PutObjectError>>
        }
    }

    pub fn raw_upload(
        &self,
        bucket: String,
        key: String,
        content_type: Option<String>,
        bytes: Vec<u8>,
    ) -> Box<Future<Item = PutObjectOutput, Error = PutObjectError>> {
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
        Box::new(
            self.inner.put_object(&request)
        )
    }

    fn prepare_image(image_type: &str, bytes: &[u8]) -> Result<HashMap<Size, Vec<u8>>, image::ImageError> {
        let mut hash:  HashMap<Size, Vec<u8>> = HashMap::new();
        let img = image::load_from_memory_with_format(bytes, image::ImageFormat::PNG)?;
        let (w, h) = img.dimensions();
        let color = img.color();
        let smallest_dimension = if w < h { w } else { h };
        if smallest_dimension == 0 { return Err(image::ImageError::DimensionError); }
        vec![Size::Thumb, Size::Small, Size::Medium, Size::Large].iter().for_each(|size| {
            let size = size.clone();
            let size2 = size.clone();
            let int_size = size as u32;
            let width = ((w as f32) * (int_size as f32) / (smallest_dimension as f32)).round() as u32;
            let height = ((h as f32) * (int_size as f32) / (smallest_dimension as f32)).round() as u32;
            let resized_image = img.resize_exact(width, height, image::FilterType::Triangle);
            let mut buffer = Vec::new();
            let _ = resized_image.save(&mut buffer, image::ImageFormat::PNG);
            hash.insert(size2, buffer);
        });

        Ok(hash)
    }

    fn encode_for_aws(s: &str) -> String {
        let s = s.replace("+", "A");
        let s = s.replace("/", "B");
        s.replace("=", "C")
    }
}
