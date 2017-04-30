//! Deserializer for the Caption type.

use std::fmt;

use serde::de::{self, Deserialize, Visitor};

use super::super::{Caption, Color, HAlign};


const FIELDS: &'static [&'static str] = &["text", "align", "valign", "color"];

pub const DEFAULT_COLOR: Color = Color(0xff, 0xff, 0xff);
pub const DEFAULT_HALIGN: HAlign = HAlign::Center;


impl<'de> Deserialize<'de> for Caption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: de::Deserializer<'de>
    {
        deserializer.deserialize_map(CaptionVisitor)
    }
}

struct CaptionVisitor;
impl<'de> Visitor<'de> for CaptionVisitor {
    type Value = Caption;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "representation of an image macro's caption")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
        where V: de::MapAccess<'de>
    {
        let mut text = None;
        let mut halign = None;
        let mut valign = None;
        let mut color = None;

        while let Some(key) = map.next_key::<String>()? {
            let key = key.trim().to_lowercase();
            match key.as_str() {
                "text" => {
                    if text.is_some() {
                        return Err(de::Error::duplicate_field("text"));
                    }
                    text = Some(map.next_value()?);
                }
                "align" | "halign" => {
                    if halign.is_some() {
                        return Err(de::Error::duplicate_field("align"));
                    }
                    halign = Some(map.next_value()?)
                }
                "valign" => {
                    if valign.is_some() {
                        return Err(de::Error::duplicate_field("valign"));
                    }
                    valign = Some(map.next_value()?)
                }
                "color" => {
                    if color.is_some() {
                        return Err(de::Error::duplicate_field("color"));
                    }
                    color = Some(map.next_value()?)
                }
                key => return Err(de::Error::unknown_field(key, FIELDS)),
            }
        }

        let text = text.ok_or_else(|| de::Error::missing_field("text"))?;
        let halign = halign.unwrap_or(DEFAULT_HALIGN);
        let valign = valign.ok_or_else(|| de::Error::missing_field("valign"))?;
        let color = color.unwrap_or(DEFAULT_COLOR);
        Ok(Caption{
            text: text,
            halign: halign,
            valign: valign,
            color: color,
        })
    }
}
