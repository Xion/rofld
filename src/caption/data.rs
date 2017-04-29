//! Defines the input data for captioning.

use std::collections::HashMap;
use std::fmt;

use serde::de::{self, Deserialize, Unexpected, Visitor};

use super::fonts;


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
#[derive(Clone, Deserialize, PartialEq)]
pub struct Caption {
    // TODO: allow to customize font on per-caption basis
    // TODO: text color & outline color
    pub text: String,
    pub align: HAlign,
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
            align: HAlign::Center,
            valign: VAlign::Bottom,
        }
    }
}
impl fmt::Debug for Caption {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{:?}{:?}({:?})", self.valign, self.align, self.text)
    }
}


// ImageMacro deserializer

impl<'de> Deserialize<'de> for ImageMacro {
    fn deserialize<D>(deserializer: D) -> Result<ImageMacro, D::Error>
        where D: de::Deserializer<'de>
    {
        deserializer.deserialize_map(ImageMacroVisitor)
    }
}

struct ImageMacroVisitor;
impl<'de> Visitor<'de> for ImageMacroVisitor {
    type Value = ImageMacro;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "valid representation of an image macro")
    }

    fn visit_map<V>(self, mut map: V) -> Result<ImageMacro, V::Error>
        where V: de::MapAccess<'de>
    {
        let mut template = None;
        let mut width = None;
        let mut height = None;
        let mut font = None;

        let mut simple_captions: HashMap<VAlign, Caption> = HashMap::new();
        let mut full_captions = vec![];

        while let Some(key) = map.next_key()? {
            let key: String = key;  // Rust is silly and needs a type hint here
            let key = key.trim().to_lowercase();

            match key.as_str() {
                // Data that's typically expected (or even mandatory).
                "template" => {
                    if template.is_some() {
                        return Err(de::Error::duplicate_field("template"));
                    }
                    template = Some(map.next_value()?);
                }
                "width" => {
                    if width.is_some() {
                        return Err(de::Error::duplicate_field("width"));
                    }
                    width = Some(map.next_value()?);
                }
                "height" => {
                    if height.is_some() {
                        return Err(de::Error::duplicate_field("height"));
                    }
                    height = Some(map.next_value()?);
                }
                "font" => {
                    if font.is_some() {
                        return Err(de::Error::duplicate_field("font"));
                    }
                    font = Some(map.next_value()?);
                }

                // Simplified way of defining top/middle/bottom captions.
                "top_text" => {
                    let mut caption = simple_captions.entry(VAlign::Top)
                        .or_insert_with(Caption::default);
                    caption.text = map.next_value()?;
                }
                "middle_text" => {
                    let mut caption = simple_captions.entry(VAlign::Middle)
                        .or_insert_with(Caption::default);
                    caption.text = map.next_value()?;
                }
                "bottom_text" => {
                    let mut caption = simple_captions.entry(VAlign::Bottom)
                        .or_insert_with(Caption::default);
                    caption.text = map.next_value()?;
                }
                "top_align" => {
                    let mut caption = simple_captions.entry(VAlign::Top)
                        .or_insert_with(Caption::default);
                    caption.align = map.next_value()?;
                }
                "middle_align" => {
                    let mut caption = simple_captions.entry(VAlign::Middle)
                        .or_insert_with(Caption::default);
                    caption.align = map.next_value()?;
                }
                "bottom_align" => {
                    let mut caption = simple_captions.entry(VAlign::Bottom)
                        .or_insert_with(Caption::default);
                    caption.align = map.next_value()?;
                }

                // Fully featured caption definition.
                "captions" => {
                    // TODO: make align optional when deserializing Caption (default to Center)
                    // TODO: allow captions to be a list of just text strings,
                    // interpreting it as simple captions (bottom, top/bottom or top/middle/bottom)
                    full_captions = map.next_value()?;
                }

                key => {
                    const FIELDS: &'static [&'static str] = &[
                        "template", "width", "height", "font", "captions",
                    ];
                    return Err(de::Error::unknown_field(&key, FIELDS));
                 }
            }
        }

        // The input should either use the full "captions" field,
        // or the simpler version with top/middle/bottom_test/align -- but not both.
        let mut captions;
        if simple_captions.len() > 0 && full_captions.len() > 0 {
            return Err(de::Error::custom(
                "`captions` cannot be provided along with `top/middle/bottom_text/align`"))
        }
        if simple_captions.len() > 0 {
            captions = simple_captions.into_iter()
                .map(|(valign, caption)| Caption{valign: valign, ..caption})
                .collect();
        } else if full_captions.len() > 0 {
            captions = full_captions;
        } else {
            captions = vec![];
        }
        captions.sort_by_key(|c| (c.valign, c.align));

        let template = template.ok_or_else(|| de::Error::missing_field("template"))?;
        Ok(ImageMacro{
            template: template,
            width: width,
            height: height,
            font: font,
            captions: captions,
        })
    }
}


// Deserializers for other stuff

impl<'de> Deserialize<'de> for HAlign {
    fn deserialize<D>(deserializer: D) -> Result<HAlign, D::Error>
        where D: de::Deserializer<'de>
    {
        deserializer.deserialize_str(HAlignVisitor)
    }
}

struct HAlignVisitor;
impl<'de> Visitor<'de> for HAlignVisitor {
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


impl<'de> Deserialize<'de> for VAlign {
    fn deserialize<D>(deserializer: D) -> Result<VAlign, D::Error>
        where D: de::Deserializer<'de>
    {
        deserializer.deserialize_str(VAlignVisitor)
    }
}

struct VAlignVisitor;
impl<'de> Visitor<'de> for VAlignVisitor {
    type Value = VAlign;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "'top', 'middle', or 'bottom'")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match &v.trim().to_lowercase() as &str {
            "top" => Ok(VAlign::Top),
            "middle" => Ok(VAlign::Middle),
            "bottom" => Ok(VAlign::Bottom),
            _ => Err(E::invalid_value(Unexpected::Str(v), &self)),
        }
    }
}
