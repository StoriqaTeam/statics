//! Preprocessors module contains functions for preprocessing images / videos, etc.

use std::collections::HashMap;
use futures_cpupool::CpuPool;
use futures::future::Future;
use futures::future;
use image;
use image::{DynamicImage, FilterType, GenericImage};

use super::error::S3Error;
use super::types::ImageSize;
use services::types::ImageFormat;

pub trait Image {
    /// Process image specified by format and bytes encoded in this format
    ///
    /// * `format` - either "png" or "jpg" - these are types that are supported
    /// * `bytes` - bytes representing encoded image
    ///
    /// Returns HashMap of sized and resized images encoded in PNG
    ///
    /// #Errors
    /// * `S3Error::Image` if encoding is incorrect, incl zero dimensions
    fn process(&self, format: ImageFormat, bytes: Vec<u8>) -> Box<Future<Item = HashMap<ImageSize, Vec<u8>>, Error = S3Error>>;
}

pub struct ImageImpl<'a> {
    cpu_pool: &'a CpuPool,
}

impl<'a> ImageImpl<'a> {
    pub fn new(cpu_pool: &'a CpuPool) -> Self {
        Self { cpu_pool }
    }

    /// Spawns resizing an image on a thread from a thread pool
    ///
    /// * `size` - image size for resizing
    /// * `image_type` - either "png", "jpg" or "jpeg" - these are types that are supported
    /// * `bytes` - bytes repesenting compessed image (compessed with `image_type` codec)
    fn resize_image_async(&self, size: &ImageSize, image: DynamicImage) -> Box<Future<Item = Vec<u8>, Error = S3Error>> {
        let size_clone = size.clone();
        Box::new(
            self.cpu_pool
                .spawn_fn(move || Self::resize_image(&size_clone, image)),
        )
    }

    /// Resizes an image on a thread from a thread pool
    ///
    /// * `size` - image size for resizing
    /// * `image_type` - either "png", "jpg" or "jpeg" - these are types that are supported
    /// * `bytes` - bytes repesenting compessed image (compessed with `image_type` codec)
    fn resize_image(size: &ImageSize, image: DynamicImage) -> Result<Vec<u8>, S3Error> {
        let (w, h) = image.dimensions();
        let smallest_dimension = if w < h { w } else { h };
        if smallest_dimension == 0 {
            return Err(S3Error::Image("Uploaded image size is zero".to_string()));
        }
        let int_size = size.clone() as u32;
        let width = ((w as f32) * (int_size as f32) / (smallest_dimension as f32)).round() as u32;
        let height = ((h as f32) * (int_size as f32) / (smallest_dimension as f32)).round() as u32;
        let resized_image = match int_size {
            x if x < smallest_dimension => image.resize_exact(width, height, FilterType::Triangle),
            _ => image,
        };
        let mut buffer = Vec::new();
        let _ = resized_image.save(&mut buffer, ImageFormat::PNG.into());
        Ok(buffer)
    }
}

impl<'a> Image for ImageImpl<'a> {
    fn process(&self, format: ImageFormat, bytes: Vec<u8>) -> Box<Future<Item = HashMap<ImageSize, Vec<u8>>, Error = S3Error>> {
        let image = match image::load_from_memory_with_format(&bytes, format.into()) {
            Ok(data) => data,
            Err(e) => return S3Error::Image(format!("Error parsing image with format {}: {}", format, e)).into(),
        };
        let mut futures: Vec<Box<Future<Item = (ImageSize, Vec<u8>), Error = S3Error>>> = [
            ImageSize::Thumb,
            ImageSize::Small,
            ImageSize::Medium,
            ImageSize::Large,
        ].iter()
            .map(|size| {
                let img = image.clone();
                let size_clone = size.clone();
                Box::new(
                    self.resize_image_async(size, img)
                        .map(|bytes| (size_clone, bytes)),
                ) as Box<Future<Item = (ImageSize, Vec<u8>), Error = S3Error>>
            })
            .collect();
        futures.push(Box::new(future::ok((ImageSize::Original, bytes))));
        Box::new(future::join_all(futures).map(|results| results.into_iter().collect::<HashMap<_, _>>()))
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::Read;
    use futures_cpupool::CpuPool;

    use super::*;
    #[test]
    fn test_image_process_png() {
        let original_image_bytes = read_static_file("image-328x228.png");
        let thumb_image_bytes = read_static_file("image-328x228-thumb.png");
        let small_image_bytes = read_static_file("image-328x228-small.png");
        let medium_image_bytes = read_static_file("image-328x228-medium.png");
        let large_image_bytes = read_static_file("image-328x228-large.png");

        let cpu_pool = CpuPool::new_num_cpus();
        let image = ImageImpl::new(&cpu_pool);
        let image_hash = image
            .process(ImageFormat::PNG, original_image_bytes.clone())
            .wait()
            .unwrap();

        assert_eq!(image_hash[&ImageSize::Thumb], thumb_image_bytes);
        assert_eq!(image_hash[&ImageSize::Small], small_image_bytes);
        assert_eq!(image_hash[&ImageSize::Medium], medium_image_bytes);
        assert_eq!(image_hash[&ImageSize::Large], large_image_bytes);
        assert_eq!(image_hash[&ImageSize::Original], original_image_bytes);
    }

    #[test]
    fn test_image_process_jpeg() {
        let original_image_bytes = read_static_file("image-1280x800.jpg");
        let coverted_original_image_bytes = read_static_file("image-1280x800.png");
        let thumb_image_bytes = read_static_file("image-1280x800-thumb.png");
        let small_image_bytes = read_static_file("image-1280x800-small.png");
        let medium_image_bytes = read_static_file("image-1280x800-medium.png");
        let large_image_bytes = read_static_file("image-1280x800-large.png");

        let cpu_pool = CpuPool::new_num_cpus();
        let image = ImageImpl::new(&cpu_pool);
        let image_hash = image
            .process(ImageFormat::JPG, original_image_bytes)
            .wait()
            .unwrap();

        assert_eq!(image_hash[&ImageSize::Thumb], thumb_image_bytes);
        assert_eq!(image_hash[&ImageSize::Small], small_image_bytes);
        assert_eq!(image_hash[&ImageSize::Medium], medium_image_bytes);
        assert_eq!(image_hash[&ImageSize::Large], large_image_bytes);
        assert_eq!(
            image_hash[&ImageSize::Original],
            coverted_original_image_bytes
        );
    }

    #[test]
    fn test_image_process_invalid_bytes() {
        let original_image_bytes = read_static_file("image-1280x800.jpg");
        let cpu_pool = CpuPool::new_num_cpus();
        let image = ImageImpl::new(&cpu_pool);
        let error = image
            .process(ImageFormat::PNG, original_image_bytes.clone())
            .wait()
            .err()
            .unwrap();
        match error {
            S3Error::Image(_) => (),
            e => assert!(false, format!("Expected error S3Error::Image, found {}", e)),
        }
    }

    fn read_static_file(name: &str) -> Vec<u8> {
        let mut file = File::open(format!("tests/static_files/{}", name)).unwrap();
        let mut buf = Vec::new();
        let _ = file.read_to_end(&mut buf);
        buf
    }
}
