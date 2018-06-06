//! Type aliases for service module

use failure;
use futures::future::Future;
use image::ImageFormat as CrateImageFormat;
use std::fmt;
use std::str::FromStr;

use errors::*;

/// Image encoding format
#[derive(Clone, Copy)]
pub enum ImageFormat {
    PNG,
    JPG,
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            ImageFormat::PNG => f.write_str("png"),
            ImageFormat::JPG => f.write_str("jpg"),
        }
    }
}

impl FromStr for ImageFormat {
    type Err = failure::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "png" => Ok(ImageFormat::PNG),
            "jpg" | "jpeg" => Ok(ImageFormat::JPG),
            other => Err(format_err!("Invalid image format: {}", other).context(Error::Image).into()),
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
pub type ServiceFuture<T> = Box<Future<Item = T, Error = failure::Error>>;
