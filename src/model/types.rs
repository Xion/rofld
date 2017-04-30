//! Module defining the model types.

use std::fmt;

use image::{Rgb, Rgba};

use super::constants::{DEFAULT_COLOR, DEFAULT_HALIGN, DEFAULT_FONT};


/// Describes an image macro. Used as an input structure.
#[derive(PartialEq)]
pub struct ImageMacro {
    pub template: String,
    pub width: Option<u32>,
    pub height: Option<u32>,

    pub font: Option<String>,
    pub captions: Vec<Caption>,
}

/// Describes a single piece of text rendered on the image macro.
#[derive(Clone, PartialEq)]
pub struct Caption {
    // TODO: allow to customize font on per-caption basis
    // TODO: outline color
    pub text: String,
    pub halign: HAlign,
    pub valign: VAlign,
    pub color: Color,
}

/// Horizontal alignment of text within a rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

/// Vertical alignment of text within a rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VAlign {
    Top,
    Middle,
    Bottom,
}

/// RGB color of the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color(pub u8, pub u8, pub u8);


impl ImageMacro {
    #[inline]
    pub fn has_text(&self) -> bool {
        self.captions.len() > 0 && self.captions.iter().any(|c| !c.text.is_empty())
    }

    #[inline]
    pub fn font(&self) -> &str {
        self.font.as_ref().map(|s| s.as_str()).unwrap_or(DEFAULT_FONT)
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
        fmt_opt_field!(font);

        ds.field("captions", &self.captions);

        ds.finish()
    }
}

impl Default for Caption {
    fn default() -> Self {
        Caption{
            text: String::new(),
            halign: DEFAULT_HALIGN,
            valign: VAlign::Bottom,  // arbitrary
            color: DEFAULT_COLOR,
        }
    }
}
impl fmt::Debug for Caption {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{:?}{:?}({:?})", self.valign, self.halign, self.text)
    }
}

impl Color {
    #[inline]
    pub fn gray(value: u8) -> Self {
        Color(value, value, value)
    }
}
impl Color {
    #[inline]
    pub fn invert(self) -> Self {
        let Color(r, g, b) = self;
        Color(0xff - r, 0xff - g, 0xff - b)
    }

    #[inline]
    pub fn to_rgb(&self) -> Rgb<u8> {
        let &Color(r, g, b) = self;
        Rgb{data: [r, g, b]}
    }

    #[inline]
    pub fn to_rgba(&self, alpha: u8) -> Rgba<u8> {
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
