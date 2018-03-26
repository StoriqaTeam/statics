//! Preprocessors module contains functions for preprocessing images / videos, etc.

use std::collections::HashMap;
use futures_cpupool::CpuPool;
use futures::future::{Future};
use futures::future;
use image;
use image::{DynamicImage, ImageFormat, FilterType, GenericImage};

use super::error::S3Error;
use super::types::ImageSize;

pub trait Image {
    fn process(&self, image_type: &str, bytes: Vec<u8>) -> Box<Future<Item = HashMap<ImageSize, Vec<u8>>, Error = S3Error>>;
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
    fn resize_image_async(
        &self,
        size: &ImageSize,
        image: DynamicImage,
    ) -> Box<Future<Item = Vec<u8>, Error = S3Error>> {
        let size_clone = size.clone();
            Box::new(
                self.cpu_pool.spawn_fn(move || {
                    Self::resize_image(&size_clone, image)
                })
            )
    }

    /// Resizes an image on a thread from a thread pool
    ///
    /// * `size` - image size for resizing
    /// * `image_type` - either "png", "jpg" or "jpeg" - these are types that are supported
    /// * `bytes` - bytes repesenting compessed image (compessed with `image_type` codec)
    fn resize_image(
        size: &ImageSize,
        image: DynamicImage,
    ) -> Result<Vec<u8>, S3Error> {
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
        let _ = resized_image.save(&mut buffer, ImageFormat::PNG);
        Ok(buffer)
    }
}

impl<'a> Image for ImageImpl<'a> {
    fn process(&self, image_type: &str, bytes: Vec<u8>) -> Box<Future<Item = HashMap<ImageSize, Vec<u8>>, Error = S3Error>> {
         let image_format = match image_type {
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
        let mut futures: Vec<Box<Future<Item = (ImageSize, Vec<u8>), Error = S3Error>>> = [ImageSize::Thumb, ImageSize::Small, ImageSize::Medium, ImageSize::Large].iter().map(|size| {
            let img = image.clone();
            let size_clone = size.clone();
            Box::new(
                self.resize_image_async(size, img).map(|bytes| (size_clone, bytes))
            ) as Box<Future<Item = (ImageSize, Vec<u8>), Error = S3Error>>
        }).collect();
        futures.push(Box::new(future::ok((ImageSize::Original, bytes))));
        Box::new(
            future::join_all(futures).map(|results| results.into_iter().collect::<HashMap<_, _>>())
        )
    }
}

