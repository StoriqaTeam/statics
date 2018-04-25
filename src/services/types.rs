//! Type aliases for service module

use futures::future::Future;
use image::ImageFormat as CrateImageFormat;
use std::fmt::{Display, Error, Formatter};
use std::str::FromStr;

use super::error::ServiceError;

/// Image encoding format
#[derive(Clone, Copy)]
pub enum ImageFormat {
    PNG,
    JPG,
}

impl Display for ImageFormat {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match *self {
            ImageFormat::PNG => f.write_str("png"),
            ImageFormat::JPG => f.write_str("jpg"),
        }
    }
}

impl FromStr for ImageFormat {
    type Err = ServiceError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "png" => Ok(ImageFormat::PNG),
            "jpg" | "jpeg" => Ok(ImageFormat::JPG),
            format => Err(ServiceError::Image(format!(
                "Invalid image format: {}",
                format
            ))),
        }
    }
}

impl Into<CrateImageFormat> for ImageFormat {
    fn into(self) -> CrateImageFormat {
        match self {
            ImageFormat::PNG => CrateImageFormat::PNG,
            ImageFormat::JPG => CrateImageFormat::JPEG,
        }
    }
}

/// Service layer Future
pub type ServiceFuture<T> = Box<Future<Item = T, Error = ServiceError>>;
