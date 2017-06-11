//! Defines the output of a captioning operation.

use image::ImageFormat;
use mime::{self, Mime};


/// Output of the captioning process.
#[derive(Clone, Debug)]
pub struct CaptionOutput {
    pub format: ImageFormat,
    pub bytes: Vec<u8>,
}

impl CaptionOutput {
    #[inline]
    pub fn new(format: ImageFormat, bytes: Vec<u8>) -> Self {
        CaptionOutput{format, bytes}
    }
}

impl CaptionOutput {
    /// The MIME type that matches output's format.
    pub fn mime_type(&self) -> Option<Mime> {
        match self.format {
            ImageFormat::GIF => Some(mime::IMAGE_GIF),
            ImageFormat::JPEG => Some(mime::IMAGE_JPEG),
            ImageFormat::PNG => Some(mime::IMAGE_PNG),
            _ => None,
        }
    }
}
