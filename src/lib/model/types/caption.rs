//! Module implementing the `Caption` type and its builder.

use std::error;
use std::fmt;

use model::constants::{DEFAULT_COLOR, DEFAULT_HALIGN, DEFAULT_FONT, DEFAULT_OUTLINE_COLOR,
                       MAX_CAPTION_LENGTH};
use super::align::{HAlign, VAlign};
use super::color::Color;


/// Describes a single piece of text rendered on the image macro.
///
/// Use the provided `Caption::text_at` method to create it
/// with most of the fields set to default values.
#[derive(Builder, Clone, PartialEq)]
#[builder(derive(Debug, PartialEq),
          pattern = "owned", build_fn(skip))]
pub struct Caption {
    /// Text to render.
    ///
    /// Newline characters (`"\n"`) cause the text to wrap.
    pub text: String,
    /// Horizontal alignment of the caption within the template rectangle.
    /// Default is `HAlign::Center`.
    pub halign: HAlign,
    /// Vertical alignment of the caption within the template rectangle.
    pub valign: VAlign,
    /// Name of the font to render the caption with. Defaults to `"Impact"`.
    pub font: String,  // TODO: this could be a Cow, but needs lifetime param
    /// Text color, defaults to white.
    pub color: Color,
    /// Text of the color outline, if any. Defaults to black.
    ///
    /// Pass `None` to draw the text without an outline.
    pub outline: Option<Color>,
}

impl Caption {
    /// Create an empty Caption at the particular vertical alignment.
    #[inline]
    pub fn at(valign: VAlign) -> Self {
        CaptionBuilder::default()
            .valign(valign)
            .build()
            .expect("Caption::at")
    }

    /// Create a Caption with a text at the particular vertical alignment.
    #[inline]
    pub fn text_at<S: Into<String>>(valign: VAlign, s: S) -> Self {
        CaptionBuilder::default()
            .valign(valign).text(s.into())
            .build()
            .expect("Caption::text_at")
    }
}

impl fmt::Debug for Caption {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{valign:?}{halign:?}{{{font:?} {outline}[{color}]}}({text:?})",
            text = self.text,
            halign = self.halign,
            valign = self.valign,
            font = self.font,
            color = self.color,
            outline = self.outline.map(|o| format!("{}", o)).unwrap_or_else(String::new))
    }
}


impl CaptionBuilder {
    /// Build the resulting `Caption`.
    pub fn build(self) -> Result<Caption, Error> {
        self.validate()?;
        Ok(Caption{
            // Note that we can't use #[builder(default)] if we override the build()
            // method with #[builder(build_fn)], which is why we have to put the defaults here.
            text: self.text.unwrap_or_else(String::new),
            halign: self.halign.unwrap_or(DEFAULT_HALIGN),
            valign: self.valign.unwrap(),  // mandatory
            font: self.font.unwrap_or_else(|| DEFAULT_FONT.into()),
            color: self.color.unwrap_or(DEFAULT_COLOR),
            outline: self.outline.unwrap_or(Some(DEFAULT_OUTLINE_COLOR)),
        })
    }

    #[doc(hidden)]
    fn validate(&self) -> Result<(), Error> {
        if let Some(ref text) = self.text {
            if text.len() > MAX_CAPTION_LENGTH {
                return Err(Error::TooLong(text.len()));
            }
        }
        if self.valign.is_none() {
            return Err(Error::NoVerticalAlign);
        }
        Ok(())
    }
}


/// Error while building a `Caption`.
#[derive(Clone, Debug)]
pub enum Error {
    /// No vertical alignment given.
    NoVerticalAlign,
    /// Caption text too long.
    TooLong(usize),
}

impl error::Error for Error {
    fn description(&self) -> &str { "Caption creation error" }
    fn cause(&self) -> Option<&error::Error> { None }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::NoVerticalAlign => write!(fmt, "no vertical alignment chosen"),
            Error::TooLong(l) => write!(fmt, "caption text too long: {} > {}",
                l, MAX_CAPTION_LENGTH),
        }
    }
}


#[cfg(test)]
mod tests {
    use model::{VAlign};
    use super::Caption;

    #[test]
    fn text_at() {
        let cap = Caption::text_at(VAlign::Top, "Test");
        assert_eq!(VAlign::Top, cap.valign);
        assert_eq!("Test", cap.text);
    }
}
