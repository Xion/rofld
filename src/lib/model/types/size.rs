//! Module defining the text size enum.

use std::cmp::Ordering;

use num::Float;

use super::super::constants::DEFAULT_TEXT_SIZE;


/// Size of the caption text.
#[derive(Clone, Copy, Debug)]
pub enum Size {
    /// Use fixed text size.
    Fixed(f32),
    /// Shrink the caption to fit the image.
    Shrink,
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
            (Size::Fixed(a), Size::Fixed(b)) =>
                float_cmp_equal_nans(a, b) == Ordering::Equal,
            (Size::Shrink, Size::Shrink) => true,
            _ => false,
        }
    }
}
impl Eq for Size {}

// Utility functions

/// Compare two floating point values in such a way that NaNs are treated
/// as equal to each other.
fn float_cmp_equal_nans<T: Copy + Float>(a: T, b: T) -> Ordering {
    match (a, b) {
        (x, y) if x.is_nan() && y.is_nan() => Ordering::Equal,
        (x, _) if x.is_nan() => Ordering::Greater,
        (_, y) if y.is_nan() => Ordering::Less,
        (_, _) => a.partial_cmp(&b).unwrap()
    }
}
