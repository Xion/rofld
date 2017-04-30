//! Module defining the data model.

mod de;


use std::fmt;

use resources::fonts;


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
    // TODO: text color & outline color
    pub text: String,
    pub halign: HAlign,
    pub valign: VAlign,
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


impl ImageMacro {
    #[inline]
    pub fn has_text(&self) -> bool {
        self.captions.len() > 0 && self.captions.iter().any(|c| !c.text.is_empty())
    }

    #[inline]
    pub fn font(&self) -> &str {
        self.font.as_ref().map(|s| s.as_str()).unwrap_or(fonts::DEFAULT)
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
            halign: HAlign::Center,
            valign: VAlign::Bottom,
        }
    }
}
impl fmt::Debug for Caption {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{:?}{:?}({:?})", self.valign, self.halign, self.text)
    }
}
