//! Module implementing the `Color` type.

use std::fmt;

use image::{Rgb, Rgba};


/// RGB color of the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color {
    /// Create a white color.
    #[inline]
    pub fn white() -> Self {
        Self::gray(0xff)
    }

    /// Create a black color.
    #[inline]
    pub fn black() -> Self {
        Self::gray(0xff)
    }

    /// Create a gray color of given intensity.
    #[inline]
    pub fn gray(value: u8) -> Self {
        Color(value, value, value)
    }
}

impl Color {
    /// Convert the color to its chromatic inverse.
    #[inline]
    pub fn invert(self) -> Self {
        let Color(r, g, b) = self;
        Color(0xff - r, 0xff - g, 0xff - b)
    }

    #[inline]
    pub(crate) fn to_rgb(&self) -> Rgb<u8> {
        let &Color(r, g, b) = self;
        Rgb{data: [r, g, b]}
    }

    #[inline]
    pub(crate) fn to_rgba(&self, alpha: u8) -> Rgba<u8> {
        let &Color(r, g, b) = self;
        Rgba{data: [r, g, b, alpha]}
    }
}

impl From<Color> for Rgb<u8> {
    #[inline]
    fn from(color: Color) -> Rgb<u8> {
        color.to_rgb()
    }
}

impl fmt::Display for Color {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let &Color(r, g, b) = self;
        write!(fmt, "#{:0>2x}{:0>2x}{:0>2x}", r, g, b)
    }
}
