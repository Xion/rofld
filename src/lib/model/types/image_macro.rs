//! Module implementing the `ImageMacro` type and its builder.

use std::error;
use std::fmt;

use model::constants::{MAX_CAPTION_COUNT, MAX_CAPTION_LENGTH, MAX_HEIGHT, MAX_WIDTH};
use super::align::{HAlign, VAlign};
use super::caption::Caption;


/// Describes an image macro. Used as an input structure.
///
/// *Note*: If `width` or `height` is provided, the result will be resized
/// whilst preserving the original aspect ratio of the template.
/// This means the final size of the image may be smaller than requested.
#[derive(Clone, Default, Eq)]
pub struct ImageMacro {
    /// Name of the template used by this image macro.
    pub template: String,
    /// Width of the rendered macro (if it is to be different from the template).
    pub width: Option<u32>,
    /// Height of the rendered macro (if it is to be different from the template).
    pub height: Option<u32>,
    /// Text captions to render over the template.
    pub captions: Vec<Caption>,
}

impl ImageMacro {
    /// Whether the image macro includes any text.
    #[inline]
    pub fn has_text(&self) -> bool {
        self.captions.len() > 0 && self.captions.iter().any(|c| !c.text.is_empty())
    }
}

impl PartialEq<ImageMacro> for ImageMacro {
    /// Check equality with another ImageMacro.
    /// This is implemented not to take the order of Captions into account.
    fn eq(&self, other: &Self) -> bool {
        self.template == other.template &&
        self.width == other.width &&
        self.height == other.height &&
        // O(n^2), I know.
        self.captions.iter().all(|c1| other.captions.iter().any(|c2| c1 == c2))
        // TODO: consider implementing captions as HashSet for this reason
    }
}

impl fmt::Debug for ImageMacro {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut ds = fmt.debug_struct("ImageMacro");
        ds.field("template", &self.template);

        macro_rules! fmt_opt_field {
            ($name:ident) => (
                if let Some(ref $name) = self.$name {
                    ds.field(stringify!($name), $name);
                }
            );
        }
        fmt_opt_field!(width);
        fmt_opt_field!(height);

        if self.captions.len() > 0 {
            ds.field("captions", &self.captions);
        }

        ds.finish()
    }
}


/// Builder for `ImageMacro`.
#[derive(Debug, Default, PartialEq)]
#[must_use = "unused builder which must be used"]
pub struct Builder {
    template: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    captions: Vec<Caption>,
}

impl Builder {
    /// Create a new `Builder` for an `ImageMacro`.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Builder {
    /// Set the template used by resulting `ImageMacro`.
    #[inline]
    pub fn template<S: Into<String>>(mut self, template: S) -> Self {
        self.template = Some(template.into()); self
    }

    /// Change the width of the resulting image macro.
    ///
    /// Note that any resizing done during rendering of the result `ImageMacro`
    /// (whether due to custom `width` or `height`) will preserve
    /// the original aspect of the template.
    ///
    /// By default, the width of the template will be used.
    #[inline]
    pub fn width(mut self, width: u32) -> Self {
        self.width = Some(width); self
    }

    /// Reset the `ImageMacro` width back to the width of the template.
    #[inline]
    pub fn clear_width(mut self) -> Self {
        self.width = None; self
    }

    /// Change the height of the resulting image macro.
    ///
    /// Note that any resizing done during rendering of the result `ImageMacro`
    /// (whether due to custom `width` or `height`) will preserve
    /// the original aspect of the template.
    ///
    /// By default, the height of the template will be used.
    #[inline]
    pub fn height(mut self, height: u32) -> Self {
        self.height = Some(height); self
    }

    /// Reset the `ImageMacro` height back to the height of the template.
    #[inline]
    pub fn clear_height(mut self) -> Self {
        self.height = None; self
    }
}

// Captioning interface.
impl Builder {
    /// Add a `Caption` to the resulting `ImageMacro`.
    #[inline]
    pub fn caption(mut self, caption: Caption) -> Self {
        self.captions.push(caption); self
    }

    /// Add a caption with given text and alignment to the resulting `ImageMacro`.
    #[inline]
    pub fn text_at<S: Into<String>>(self, valign: VAlign, halign: HAlign, text: S) -> Self {
        self.caption(Caption {
            halign: halign,
            ..Caption::text_at(valign, text)
        })
    }

    /// Add a centered text caption of given vertical alignment to the `ImageMacro`.
    #[inline]
    pub fn centered_text_at<S: Into<String>>(self, valign: VAlign, text: S) -> Self {
        self.caption(Caption::text_at(valign, text))
    }

    // TODO: top_text, middle_text, bottom_text (with halign center)
    // TODO: top_left_text, top_center_text, etc.
}

impl Builder {
    /// Build the resulting `ImageMacro`.
    #[inline]
    pub fn build(self) -> Result<ImageMacro, Error> {
        self.validate()?;
        Ok(ImageMacro{
            template: self.template.unwrap(),
            width: self.width,
            height: self.height,
            captions: self.captions,
        })
    }

    #[doc(hidden)]
    fn validate(&self) -> Result<(), Error> {
        if self.template.is_none() {
            return Err(Error::NoTemplate);
        }

        let width = self.width.unwrap_or(0);
        let height = self.height.unwrap_or(0);
        if !(width <= MAX_WIDTH && height <= MAX_HEIGHT) {
            return Err(Error::TooLarge(self.width, self.height));
        }

        if self.captions.len() > MAX_CAPTION_COUNT {
            return Err(Error::TooManyCaptions(self.captions.len()));
        }
        for cap in &self.captions {
            if cap.text.len() > MAX_CAPTION_LENGTH {
                return Err(Error::CaptionTooLong(cap.text.len()));
            }
        }

        Ok(())
    }
}


/// Error while building an `ImageMacro`.
#[derive(Clone, Debug)]
pub enum Error {
    /// No template given.
    NoTemplate,
    /// Requested image size is too large.
    TooLarge(Option<u32>, Option<u32>),
    /// Too many captions.
    TooManyCaptions(usize),
    /// Caption text too long.
    CaptionTooLong(usize),
}

impl error::Error for Error {
    fn description(&self) -> &str { "ImageMacro creation error" }
    fn cause(&self) -> Option<&error::Error> { None }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::NoTemplate => write!(fmt, "no template chosen"),
            Error::TooLarge(w, h) => write!(fmt, "target image too large: {}x{} > {}x{}",
                w.map(|w| format!("{}", w)).as_ref().map(|s| s.as_str()).unwrap_or("(default)"),
                h.map(|h| format!("{}", h)).as_ref().map(|s| s.as_str()).unwrap_or("(default)"),
                MAX_WIDTH, MAX_HEIGHT),
            Error::TooManyCaptions(c) =>
                write!(fmt, "too many captions: {} > {}", c, MAX_CAPTION_COUNT),
            Error::CaptionTooLong(l) =>
                write!(fmt, "caption too long: {} > {}", l, MAX_CAPTION_LENGTH),
        }
    }
}
