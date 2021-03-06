//! Deserializer for the Caption type.

use std::fmt;

use serde::de::{self, Deserialize, Visitor, Unexpected};

use super::super::{Caption, Size,
                   DEFAULT_FONT, DEFAULT_HALIGN, DEFAULT_COLOR,
                   DEFAULT_OUTLINE_COLOR, DEFAULT_TEXT_SIZE};


const FIELDS: &'static [&'static str] = &[
    "text", "align", "valign", "font", "color", "outline", "size",
];
const REQUIRED_FIELDS_COUNT: usize = 2;  // text & valign

const EXPECTING_MSG: &'static str = "map or struct with image macro caption";
lazy_static! {
    static ref EXPECTING_FIELD_COUNT_MSG: String = format!(
        "at least {} and no more than {}", REQUIRED_FIELDS_COUNT, FIELDS.len());
}


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
        write!(fmt, "{}", EXPECTING_MSG)
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
        where V: de::MapAccess<'de>
    {
        // Preemptively check for length against minimum & maximum.
        if let Some(size) = map.size_hint() {
            if size < REQUIRED_FIELDS_COUNT || size > FIELDS.len() {
                return Err(de::Error::invalid_length(
                    size, &(&*EXPECTING_FIELD_COUNT_MSG as &str)));
            }
        }

        let mut text = None;
        let mut halign = None;
        let mut valign = None;
        let mut font = None;
        let mut color = None;
        let mut outline: Option<Option<_>> = None;
        let mut size = None;

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
                "size" => {
                    if size.is_some() {
                        return Err(de::Error::duplicate_field("size"));
                    }
                    size = Some(map.next_value()?);
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
        let size = size.unwrap_or_else(|| Size::Fixed(DEFAULT_TEXT_SIZE));

        Ok(Caption{text, halign, valign, font, color, outline, size})
    }
}


#[cfg(test)]
mod tests {
    mod generic {
        use itertools::Itertools;
        use serde_test::{assert_de_tokens, assert_de_tokens_error, Token as T};
        use ::model::{Color, HAlign, VAlign};
        use super::super::{Caption, EXPECTING_FIELD_COUNT_MSG, EXPECTING_MSG, FIELDS};

        lazy_static! {
            static ref EXPECTING_FIELD_MSG: String = format!("one of {}",
                FIELDS.iter().format_with(", ", |x, f| f(&format_args!("`{}`", x))));
        }

        #[test]
        fn must_be_map() {
            assert_de_tokens_error::<Caption>(
                &[T::Unit],
                &format!("invalid type: unit value, expected {}", EXPECTING_MSG));
            assert_de_tokens_error::<Caption>(
                &[T::Bool(true)],
                &format!("invalid type: boolean `true`, expected {}", EXPECTING_MSG));
            assert_de_tokens_error::<Caption>(
                &[T::I32(42)],
                &format!("invalid type: integer `42`, expected {}", EXPECTING_MSG));
            assert_de_tokens_error::<Caption>(
                &[T::Char(0x42 as char)],
                &format!(r#"invalid type: string "B", expected {}"#, EXPECTING_MSG));
            assert_de_tokens_error::<Caption>(
                &[T::Tuple { len: 1 }, T::Str("foo")],
                &format!("invalid type: sequence, expected {}", EXPECTING_MSG));
            // String is possible only when deserializing as part of the ImageMacro;
            // otherwise we won't have any sensible default for valign.
            assert_de_tokens_error::<Caption>(
                &[T::Str("test")],
                &format!(r#"invalid type: string "test", expected {}"#, EXPECTING_MSG));
            assert_de_tokens_error::<Caption>(
                &[T::String("test")],
                &format!(r#"invalid type: string "test", expected {}"#, EXPECTING_MSG));
        }

        #[test]
        fn must_have_required_fields() {
            assert_de_tokens_error::<Caption>(
                &[T::Map{len: Some(1)}],
                &format!("invalid length 1, expected {}", *EXPECTING_FIELD_COUNT_MSG));
            assert_de_tokens_error::<Caption>(
                &[T::Map { len: None }, T::MapEnd],
                "missing field `text`");
            assert_de_tokens_error::<Caption>(&[
                T::Map { len: None },
                T::Str("something"), T::Str("or other"),
            ], &format!("unknown field `something`, expected {}", *EXPECTING_FIELD_MSG));
            assert_de_tokens_error::<Caption>(&[
                T::Map { len: None },
                T::Str("text"), T::Str("very caption"),
                T::MapEnd,
            ], "missing field `valign`");

            assert_de_tokens(&Caption::text_at(VAlign::Top, "Test"), &[
                T::Map { len: None },
                T::Str("text"), T::Str("Test"),
                T::Str("valign"), T::Enum{name: "VAlign"}, T::Str("top"), T::Unit,
                T::MapEnd,
            ]);
            assert_de_tokens_error::<Caption>(&[
                T::Map { len: None },
                T::Str("text"), T::Str(""),
            ], r#"invalid value: string "", expected non-empty string"#);
        }

        #[test]
        fn can_have_optional_fields() {
            assert_de_tokens(
                &Caption{halign: HAlign::Center, ..Caption::text_at(VAlign::Top, "Test")},
                &[
                    T::Map { len: None },
                    T::Str("text"), T::Str("Test"),
                    T::Str("valign"), T::Enum{name: "VAlign"}, T::Str("top"), T::Unit,
                    T::Str("halign"), T::Enum{name: "HAlign"}, T::Str("center"), T::Unit,
                    T::MapEnd,
                ]);
            assert_de_tokens(
                &Caption{font: "Comic Sans".into(), ..Caption::text_at(VAlign::Top, "Test")},
                &[
                    T::Map { len: None },
                    T::Str("text"), T::Str("Test"),
                    T::Str("valign"), T::Enum{name: "VAlign"}, T::Str("top"), T::Unit,
                    T::Str("font"), T::BorrowedStr("Comic Sans"),
                    T::MapEnd,
                ]);
            assert_de_tokens(
                &Caption{color: Color(1, 2, 3), ..Caption::text_at(VAlign::Top, "Test")},
                &[
                    T::Map { len: None },
                    T::Str("text"), T::Str("Test"),
                    T::Str("valign"), T::Enum{name: "VAlign"}, T::Str("top"), T::Unit,
                    T::Str("color"), T::Seq { len: Some(3) }, T::U8(1), T::U8(2), T::U8(3), T::SeqEnd,
                    T::MapEnd,
                ]);
            // But not too many.
            assert_de_tokens_error::<Caption>(
                &[T::Map{len: Some(9)}],
                &format!("invalid length 9, expected {}", *EXPECTING_FIELD_COUNT_MSG));
        }

        #[test]
        fn can_have_null_outline() {
            assert_de_tokens(
                &Caption{outline: None, ..Caption::text_at(VAlign::Top, "Test")},
                &[
                    T::Map { len: None },
                    T::Str("text"), T::Str("Test"),
                    T::Str("valign"), T::Enum{name: "VAlign"}, T::Str("top"), T::Unit,
                    T::Str("outline"), T::None,
                    T::MapEnd,
                ]);
        }
    }

    mod json {
        use serde_json::from_value as from_json;
        use spectral::prelude::*;
        use ::model::{Color, Caption, DEFAULT_OUTLINE_COLOR};

        #[test]
        fn required_fields() {
            assert_that!(from_json::<Caption>(json!({"text": "Test"})))
                .is_err().matches(|e| format!("{}", e).contains("at least"));
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

    // TODO: tests for "size" field
}
