//! Defines the output of a captioning operation.

use std::ops::Deref;

use image::ImageFormat;
use mime::{self, Mime};


/// Output of the captioning process.
#[derive(Clone, Debug)]
#[must_use = "unused caption output which must be used"]
pub struct CaptionOutput {
    format: ImageFormat,
    bytes: Vec<u8>,
}

impl CaptionOutput {
    #[inline]
    pub(super) fn new(format: ImageFormat, bytes: Vec<u8>) -> Self {
        CaptionOutput{format, bytes}
    }
}

impl CaptionOutput {
    /// Image format of the output.
    #[inline]
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// Raw bytes of the output.
    ///
    /// See `CaptionOutput::format` for how to interpret it.
    #[inline]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..]
    }

    /// Convert the output into a vector of bytes.
    #[inline]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Convert the output into boxed slice of bytes.
    #[inline]
    pub fn into_boxed_bytes(self) -> Box<[u8]> {
        self.bytes.into_boxed_slice()
    }

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

impl Deref for CaptionOutput {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.bytes()
    }
}

impl Into<Vec<u8>> for CaptionOutput {
    fn into(self) -> Vec<u8> {
        self.into_bytes()
    }
}
impl Into<Box<[u8]>> for CaptionOutput {
    fn into(self) -> Box<[u8]> {
        self.into_boxed_bytes()
    }
}
