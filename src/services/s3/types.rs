//! Shared types for s3 service

use std::fmt::{Display, Formatter, Error};

/// Image sizes that will go to s3 for traffic optimization
#[derive(PartialEq, Eq, Hash, Clone)]
pub enum ImageSize {
    Thumb = 40,
    Small = 80,
    Medium = 320,
    Large = 640,
    /// Original means don't resize
    Original = 0
}

impl Display for ImageSize {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            &ImageSize::Thumb => f.write_str("thumb"),
            &ImageSize::Small => f.write_str("small"),
            &ImageSize::Medium => f.write_str("medium"),
            &ImageSize::Large => f.write_str("large"),
            &ImageSize::Original => f.write_str("original"),
        }
    }
}
