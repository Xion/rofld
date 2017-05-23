//! Module defining the model types.

use std::fmt;

use image::{Rgb, Rgba};

use super::constants::{DEFAULT_COLOR, DEFAULT_OUTLINE_COLOR, DEFAULT_HALIGN, DEFAULT_FONT};


/// Describes an image macro. Used as an input structure.
#[derive(Default)]
pub struct ImageMacro {
    pub template: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub captions: Vec<Caption>,
}

/// Describes a single piece of text rendered on the image macro.
#[derive(Clone, PartialEq)]
pub struct Caption {
    pub text: String,
    pub halign: HAlign,
    pub valign: VAlign,
    pub font: String,  // TODO: this could be a Cow, but needs lifetime param
    pub color: Color,
    pub outline: Option<Color>,
}

macro_attr! {
    /// Horizontal alignment of text within a rectangle.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
             Deserialize, IterVariants!(HAligns))]
    #[serde(rename_all = "lowercase")]
    pub enum HAlign {
        Left,
        Center,
        Right,
    }
}

macro_attr! {
    /// Vertical alignment of text within a rectangle.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
             Deserialize, IterVariants!(VAligns))]
    #[serde(rename_all = "lowercase")]
    pub enum VAlign {
        Top,
        Middle,
        Bottom,
    }
}

/// RGB color of the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color(pub u8, pub u8, pub u8);


impl ImageMacro {
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

impl Caption {
    /// Create an empty Caption at the particular vertical alignment.
    #[inline]
    pub fn at(valign: VAlign) -> Self {
        Self::text_at(valign, "")
    }

    /// Create a Caption with a text at the particular vertical alignment.
    #[inline]
    pub fn text_at<S: Into<String>>(valign: VAlign, s: S) -> Self {
        Caption{
            text: s.into(),
            halign: DEFAULT_HALIGN,
            valign: valign,
            font: DEFAULT_FONT.into(),
            color: DEFAULT_COLOR,
            outline: Some(DEFAULT_OUTLINE_COLOR),
        }
    }
}
impl fmt::Debug for Caption {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{valign:?}{halign:?}{{{font:?} {outline}[{color}]}}({text:?})",
            text=self.text,
            halign=self.halign,
            valign=self.valign,
            font=self.font,
            color=self.color,
            outline=self.outline.map(|o| format!("{}", o)).unwrap_or_else(String::new))
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
impl fmt::Display for Color {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let &Color(r, g, b) = self;
        write!(fmt, "#{:0>2x}{:0>2x}{:0>2x}", r, g, b)
    }
}
