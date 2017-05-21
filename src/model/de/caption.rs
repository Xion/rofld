//! Deserializer for the Caption type.

use std::fmt;

use serde::de::{self, Deserialize, Visitor, Unexpected};

use super::super::{Caption, DEFAULT_FONT, DEFAULT_HALIGN, DEFAULT_COLOR, DEFAULT_OUTLINE_COLOR};


const FIELDS: &'static [&'static str] = &[
    "text", "align", "valign", "font", "color", "outline",
];


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
        let mut font = None;
        let mut color = None;
        let mut outline: Option<Option<_>> = None;

        while let Some(key) = map.next_key::<String>()? {
            let key = key.trim().to_lowercase();
            match key.as_str() {
                "text" => {
                    if text.is_some() {
                        return Err(de::Error::duplicate_field("text"));
                    }
                    let value: String = map.next_value()?;
                    if value.is_empty() {
                        return Err(de::Error::invalid_value(
                            Unexpected::Str(&value), &"non-empty string"));
                    }
                    text = Some(value);
                }
                "align" | "halign" => {
                    if halign.is_some() {
                        return Err(de::Error::duplicate_field("align"));
                    }
                    halign = Some(map.next_value()?);
                }
                "valign" => {
                    if valign.is_some() {
                        return Err(de::Error::duplicate_field("valign"));
                    }
                    valign = Some(map.next_value()?);
                }
                "font" => {
                    if font.is_some() {
                        return Err(de::Error::duplicate_field("font"));
                    }
                    font = Some(map.next_value()?);
                }
                "color" => {
                    if color.is_some() {
                        return Err(de::Error::duplicate_field("color"));
                    }
                    color = Some(map.next_value()?);
                }
                "outline" => {
                    // If "outline" is not provided, the default outline color is used.
                    // It can also be provided but null, in which case there shall be
                    // no text outline.
                    if outline.is_some() {
                        return Err(de::Error::duplicate_field("outline"));
                    }
                    outline = Some(map.next_value()?);
                }
                key => return Err(de::Error::unknown_field(key, FIELDS)),
            }
        }

        let text = text.ok_or_else(|| de::Error::missing_field("text"))?;
        let halign = halign.unwrap_or(DEFAULT_HALIGN);
        let valign = valign.ok_or_else(|| de::Error::missing_field("valign"))?;
        let font = font.unwrap_or(DEFAULT_FONT).into();
        let color = color.unwrap_or(DEFAULT_COLOR);
        let outline = outline.unwrap_or_else(|| Some(DEFAULT_OUTLINE_COLOR));

        Ok(Caption{text, halign, valign, font, color, outline})
    }
}


#[cfg(test)]
mod tests {
    use serde_json::from_value as from_json;
    use spectral::prelude::*;
    use ::model::{Color, Caption, DEFAULT_OUTLINE_COLOR};

    #[test]
    fn required_fields() {
        assert_that!(from_json::<Caption>(json!({"text": "Test"})))
            .is_err().matches(|e| format!("{}", e).contains("valign"));
        assert_that!(from_json::<Caption>(json!({"halign": "left", "valign": "top"})))
            .is_err().matches(|e| format!("{}", e).contains("text"));
        // Text cannot be empty.
        assert_that!(from_json::<Caption>(json!({"text": "", "valign": "center"})))
            .is_err().matches(|e| format!("{}", e).contains("non-empty string"));
    }

    #[test]
    fn default_outline() {
        let caption = json!({"text": "Test", "valign": "top"});
        assert_that!(from_json::<Caption>(caption)).is_ok()
            .map(|c| &c.outline).is_some().is_equal_to(&DEFAULT_OUTLINE_COLOR);
    }

    /// Test that the default outline color is used even when custom "color" is provided.
    ///
    /// Historically, we would invert "color" in this case,
    /// but this is too cumbersome to keep consistent between different ways both colors
    /// can be provided in ImageMacro.
    #[test]
    fn default_outline_around_non_default_color() {
        let caption = json!({"text": "Test", "valign": "top", "color": [0, 0, 255]});
        assert_that!(from_json::<Caption>(caption)).is_ok()
            .map(|c| &c.outline).is_some().is_equal_to(&DEFAULT_OUTLINE_COLOR);
    }

    #[test]
    fn outline_custom_color() {
        let caption = json!({"text": "Test", "valign": "top", "outline": "red"});
        assert_that!(from_json::<Caption>(caption)).is_ok()
            .map(|c| &c.outline).is_some().is_equal_to(&Color(0xff, 0x0, 0x0));
    }

    #[test]
    fn outline_disabled_if_null() {
        let caption = json!({"text": "Test", "valign": "top", "outline": null});
        assert_that!(from_json::<Caption>(caption)).is_ok()
            .map(|c| &c.outline).is_none();
    }
}
