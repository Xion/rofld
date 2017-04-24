//! Defines the input data for captioning.

use std::fmt;

use serde::de::{self, Deserialize, Unexpected, Visitor};

use super::fonts;


/// Describes an image macro. Used as an input structure.
#[derive(Deserialize, PartialEq)]
pub struct ImageMacro {
    pub template: String,
    pub width: Option<u32>,
    pub height: Option<u32>,

    pub font: Option<String>,
    pub top_text: Option<String>,
    pub top_align: Option<HAlign>,
    pub middle_text: Option<String>,
    pub middle_align: Option<HAlign>,
    pub bottom_text: Option<String>,
    pub bottom_align: Option<HAlign>,
}

impl ImageMacro {
    #[inline]
    pub fn has_text(&self) -> bool {
        self.top_text.is_some() || self.middle_text.is_some() || self.bottom_text.is_some()
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
        fmt_opt_field!(top_text);
        fmt_opt_field!(top_align);
        fmt_opt_field!(middle_text);
        fmt_opt_field!(middle_align);
        fmt_opt_field!(bottom_text);
        fmt_opt_field!(bottom_align);

        ds.finish()
    }
}


/// Horizontal alignment of text within a rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

impl Deserialize for HAlign {
    fn deserialize<D>(deserializer: D) -> Result<HAlign, D::Error>
        where D: de::Deserializer
    {
        deserializer.deserialize_str(HAlignVisitor)
    }
}

struct HAlignVisitor;
impl Visitor for HAlignVisitor {
    type Value = HAlign;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "'left', 'center', or 'right'")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match &v.trim().to_lowercase() as &str {
            "left" => Ok(HAlign::Left),
            "center" => Ok(HAlign::Center),
            "right" => Ok(HAlign::Right),
            _ => Err(E::invalid_value(Unexpected::Str(v), &self)),
        }
    }
}
