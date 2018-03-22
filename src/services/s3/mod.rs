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
use rusoto_core::RusotoFuture;
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
            Thumb => f.write_str("thumb"),
            Small => f.write_str("small"),
            Medium => f.write_str("medium"),
            Large => f.write_str("large"),
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

    fn create_aws_name(prefix: &str, image_type: &str, size: Option<Size>) -> String {
        let mut name_bytes = vec![0; HASH_LEN_BYTES as usize];
        let buffer = name_bytes.as_mut_slice();
        rand::thread_rng().fill_bytes(buffer);
        let name = match size {
            Some(size) => format!("{}-{}-{}.{}", prefix, encode(buffer), size, image_type),
            None => format!("{}-{}.{}", prefix, encode(buffer), image_type),
        };
        name
    }

    fn upload_image_with_size(&self, size: Option<Size>, bucket: &str, content_type: &str, image_type: &str, image_hash: HashMap<Size, Vec<u8>>) -> Box<Future<Item = (), Error = PutObjectError>> {
        let bytes = image_hash.get(&Size::Thumb).unwrap().to_vec();
        let content_type = content_type.to_string();
        let self_clone = self.clone();
        let name = Self::create_aws_name("img", image_type, Some(Size::Thumb));
        self.raw_upload(bucket.to_string(), name, Some(content_type), bytes);
        unimplemented!()
    }

    pub fn upload_image(&self, image_type: &str, bytes: Vec<u8>) -> Box<Future<Item = String, Error = PutObjectError>> {
        let content_type = format!("image/{}", image_type);
        let bucket = "storiqa-dev";
        let name = Self::create_aws_name("img", image_type, None);
        let url_encoded_name = Self::url_encode_base64(&name);
        let url = format!(
            "https://s3.amazonaws.com/{}/{}",
            bucket, url_encoded_name
        );
        if let Ok(image_hash) = Self::prepare_image(image_type, &bytes[..]) {
            let thumb = image_hash.get(&Size::Thumb).unwrap().to_vec();
            let thumb_content_type = content_type.clone();
            let thumb_self_clone = self.clone();
            let thumb_name = Self::create_aws_name("img", image_type, Some(Size::Thumb));
            let small = image_hash.get(&Size::Small).unwrap().to_vec();
            let small_content_type = content_type.clone();
            let small_self_clone = self.clone();
            let small_name = Self::create_aws_name("img", image_type, Some(Size::Small));
            let medium = image_hash.get(&Size::Medium).unwrap().to_vec();
            let medium_content_type = content_type.clone();
            let medium_self_clone = self.clone();
            let medium_name = Self::create_aws_name("img", image_type, Some(Size::Medium));
            let large = image_hash.get(&Size::Large).unwrap().to_vec();
            let large_content_type = content_type.clone();
            let large_self_clone = self.clone();
            let large_name = Self::create_aws_name("img", image_type, Some(Size::Large));

            Box::new(
                self.raw_upload("storiqa-dev".to_string(), name.to_string(), Some(content_type), bytes)
                    .and_then(move |_| {
                        thumb_self_clone.raw_upload("storiqa-dev".to_string(), format!("{}-{}", Size::Thumb, thumb_name), Some(thumb_content_type), thumb)
                    })
                    .and_then(move |_| {
                        small_self_clone.raw_upload("storiqa-dev".to_string(), format!("{}-{}", Size::Thumb, small_name), Some(small_content_type), small)
                    })
                    .and_then(move |_| {
                        medium_self_clone.raw_upload("storiqa-dev".to_string(), format!("{}-{}", Size::Thumb, medium_name), Some(medium_content_type), medium)
                    })
                    .and_then(move |_| {
                        large_self_clone.raw_upload("storiqa-dev".to_string(), format!("{}-{}", Size::Thumb, large_name), Some(large_content_type), large)
                    })
                    .map(move |_| url)
            )
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
        let smallest_dimension = if w < h { w } else { h };
        if smallest_dimension == 0 { return Err(image::ImageError::DimensionError); }
        vec![Size::Thumb, Size::Small, Size::Medium, Size::Large].iter().map(|size| {
            let size = size.clone();
            let size2 = size.clone();
            let scale = (size as u32) / smallest_dimension;
            let resized_image = img.resize(w * scale, h * scale, image::FilterType::Triangle).raw_pixels();
            hash.insert(size2, resized_image)
        });

        Ok(hash)
    }

    fn url_encode_base64(s: &str) -> String {
        let s = s.replace("+", "%2B");
        let s = s.replace("/", "%2F");
        s.replace("=", "%3D")
    }
}
