//! Defines the output of a captioning operation.

use std::io;
use std::ops::Deref;

use image::{self, DynamicImage, FilterType, GenericImage, ImageFormat};
use mime::Mime;

use util::text::{self, Style};
use super::error::CaptionError;


/// Output of the captioning process.
#[derive(Clone, Debug)]
pub struct CaptionOutput {
    pub format: ImageFormat,
    pub bytes: Vec<u8>,
}

impl CaptionOutput {
    pub fn new(format: ImageFormat, bytes: Vec<u8>) -> Self {
        CaptionOutput{format, bytes}
    }
}

impl CaptionOutput {
    pub fn mime_type(&self) -> Option<Mime> {
        match self.format {
            ImageFormat::GIF => Some(mime!(Image/Gif)),
            ImageFormat::JPEG => Some(mime!(Image/Jpeg)),
            ImageFormat::PNG => Some(mime!(Image/Png)),
            _ => None,
        }
    }
}
