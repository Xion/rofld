//! Module defining the text size enum.

use float_ord::FloatOrd;

use super::super::constants::DEFAULT_TEXT_SIZE;


/// Size of the caption text.
#[derive(Clone, Copy, Debug)]
pub enum Size {
    /// Use fixed text size.
    ///
    /// The text will be broken up into multiple lines if necessary,
    /// but its size will remain constant.
    Fixed(f32),
    /// Shrink a single line caption to fit the image.
    ///
    /// Caption text will not be broken into multiple lines
    /// and any preexisting line breaks will be ignored
    Shrink,
    /// Fit the text within the image,
    /// breaking it up and reducing its size if necessary.
    Fit,
}

impl Size {
    /// Whether this text size as been defined as fixed.
    #[inline]
    pub fn is_fixed(&self) -> bool {
        self.as_number().is_some()
    }

    /// Return the numeric text size, if specified.
    #[inline]
    pub fn as_number(&self) -> Option<f32> {
        match *self { Size::Fixed(s) => Some(s), _ => None }
    }
}

impl Default for Size {
    fn default() -> Self {
        DEFAULT_TEXT_SIZE.into()
    }
}

impl From<f32> for Size {
    fn from(input: f32) -> Self {
        Size::Fixed(input)
    }
}
impl From<f64> for Size {
    fn from(input: f64) -> Self {
        Size::from(input as f32)
    }
}

impl PartialEq for Size {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (Size::Fixed(a), Size::Fixed(b)) => FloatOrd(a).eq(&FloatOrd(b)),
            (Size::Shrink, Size::Shrink) => true,
            (Size::Fit, Size::Fit) => true,
            _ => false,
        }
    }
}
impl Eq for Size {}
