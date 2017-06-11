//! Deserializer for the Color type.

use std::fmt;
use std::str::FromStr;

use css_color_parser::{Color as CssColor, ColorParseError as CssColorParseError};
use serde::de::{self, Deserialize, Visitor};

use super::super::Color;


const FIELDS: &'static [&'static str] = &["r", "g", "b"];
const EXPECTING_MSG: &'static str = "CSS color string or array/map of RGB values";


impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: de::Deserializer<'de>
    {
        deserializer.deserialize_any(ColorVisitor)
    }
}

struct ColorVisitor;
impl<'de> Visitor<'de> for ColorVisitor {
    type Value = Color;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", EXPECTING_MSG)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        let color = Color::from_str(v).map_err(|e| {
            warn!("Failed to parse color `{}`: {}", v, e);
            E::custom(e)
        })?;
        Ok(color)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where A: de::SeqAccess<'de>
    {
        // Preemptively check for length.
        if let Some(size) = seq.size_hint() {
            if size != FIELDS.len() {
                return Err(de::Error::invalid_length(
                    size, &(&format!("{}", FIELDS.len()) as &str)));
            }
        }

        let mut channels = Vec::with_capacity(FIELDS.len());
        while let Some(elem) = seq.next_element::<u8>()? {
            channels.push(elem);

            // Immediately signal any length errors.
            if channels.len() > FIELDS.len() {
                return Err(de::Error::invalid_length(
                    channels.len(), &(&format!("{}", FIELDS.len()) as &str)));
            }
        }
        let mut result = channels.into_iter();
        Ok(Color(result.next().unwrap(),
                 result.next().unwrap(),
                 result.next().unwrap()))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where A: de::MapAccess<'de>
    {
        // Preemptively check for length.
        if let Some(size) = map.size_hint() {
            if size != FIELDS.len() {
                return Err(de::Error::invalid_length(
                    size, &(&format!("{}", FIELDS.len()) as &str)));
            }
        }

        let (mut r, mut g, mut b) = (None, None, None);
        while let Some(key) = map.next_key::<String>()? {
            let key = key.trim().to_lowercase();
            match key.as_str() {
                // TODO: consider accepting 'r'/'g'/'b' characters as keys, too
                "r" | "red" => {
                    if r.is_some() {
                        return Err(de::Error::duplicate_field("r"));
                    }
                    r = Some(map.next_value()?);
                }
                "g" | "green" => {
                    if g.is_some() {
                        return Err(de::Error::duplicate_field("g"));
                    }
                    g = Some(map.next_value()?);
                }
                "b" | "blue" => {
                    if b.is_some() {
                        return Err(de::Error::duplicate_field("b"));
                    }
                    b = Some(map.next_value()?);
                }
                key => return Err(de::Error::unknown_field(key, FIELDS)),
            }
        }

        let r = r.ok_or_else(|| de::Error::missing_field("r"))?;
        let g = g.ok_or_else(|| de::Error::missing_field("g"))?;
        let b = b.ok_or_else(|| de::Error::missing_field("b"))?;
        Ok(Color(r, g, b))
    }
}


impl FromStr for Color {
    type Err = ColorParseError;

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        // Prep the string, most notably replacing all other possible hex prefixes
        // with the standard CSS one.
        let mut s = v.trim().to_lowercase();
        let mut had_hex_prefix = false;
        for &prefix in ["#", "0x", "$"].into_iter() {
            if s.starts_with(prefix) {
                s = s.trim_left_matches(prefix).to_owned();

                // If a prefix other than the standard CSS one is used,
                // the color has to be a full 24-bit hex number.
                if prefix != "#" && s.len() != 6 {
                    return Err(ColorParseError::Css(CssColorParseError));
                }

                had_hex_prefix = true;
                break;
            }
        }
        if had_hex_prefix {
            s = format!("#{}", s);
        }

        let css_color: CssColor = s.parse()?;
        if css_color.a != 1.0 {
            return Err(ColorParseError::Alpha(css_color.a));
        }

        Ok(Color(css_color.r, css_color.g, css_color.b))
    }
}


/// Error that may occur while deserializing the Color.
#[derive(Debug, Error)]
pub enum ColorParseError {
    /// Error while trying to parse a string as CSS color.
    #[error(msg = "invalid CSS color syntax")]
    Css(CssColorParseError),
    /// Error for when the color erroneously includes an alpha channel value.
    #[error(no_from, non_std, msg =" color transparency not supported")]
    Alpha(f32),
}

// This is necessary because css_color_parser::ColorParseError doesn't impl PartialEq,
// so we cannot #[derive] that ourselves :(
impl PartialEq<ColorParseError> for ColorParseError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&ColorParseError::Css(_), &ColorParseError::Css(_)) => true,
            (&ColorParseError::Alpha(a1), &ColorParseError::Alpha(a2)) => a1 == a2,
            _ => false,
        }
    }
}


#[cfg(test)]
mod tests {
    mod generic {
        use itertools::Itertools;
        use serde_test::{assert_de_tokens, assert_de_tokens_error, Token as T};
        use super::super::{Color, EXPECTING_MSG, FIELDS};

        lazy_static! {
            static ref EXPECTING_FIELD_MSG: String = format!("one of {}",
                FIELDS.iter().format_with(", ", |x, f| f(&format_args!("`{}`", x))));
        }

        #[test]
        fn must_be_valid_type() {
            assert_de_tokens_error::<Color>(
                &[T::Unit],
                &format!("invalid type: unit value, expected {}", EXPECTING_MSG));
            assert_de_tokens_error::<Color>(
                &[T::Bool(false)],
                &format!("invalid type: boolean `false`, expected {}", EXPECTING_MSG));
        }

        #[test]
        fn can_be_css_color_name() {
            assert_de_tokens(&Color(255, 0, 0), &[T::Str("red")]);
            assert_de_tokens(&Color(255, 99, 71), &[T::Str("tomato")]);
            // Valid CSS string though.
            assert_de_tokens_error::<Color>(&[T::Str("uwotm8")], "invalid CSS color syntax");
        }

        #[test]
        fn can_be_rgb_sequence() {
            assert_de_tokens(&Color(1, 2, 3), &[
                T::Seq{len: Some(3)}, T::U8(1), T::U8(2), T::U8(3), T::SeqEnd]);
            assert_de_tokens(&Color(1, 2, 3), &[
                T::Seq{len: None}, T::U8(1), T::U8(2), T::U8(3), T::SeqEnd]);
            assert_de_tokens(&Color(1, 2, 3), &[
                T::Tuple{len: 3}, T::U8(1), T::U8(2), T::U8(3), T::TupleEnd]);
            // Must be exactly 3 elements.
            assert_de_tokens_error::<Color>(&[T::Seq{len: Some(7)}], "invalid length 7, expected 3");
            assert_de_tokens_error::<Color>(&[
                T::Seq{len: None}, T::U8(1), T::U8(2), T::U8(3), T::U8(4),
            ], "invalid length 4, expected 3");
        }

        #[test]
        #[should_panic(expected = "remaining tokens")]
        fn cannot_be_too_long_rgb_sequence() {
            // This will signal error at 4th token but then serde_test will panic.
            assert_de_tokens_error::<Color>(&[
                T::Seq{len: None}, T::U8(1), T::U8(2), T::U8(3), T::U8(4), T::U8(5), T::U8(6),
            ], "invalid length 4, expected 3");
        }

        #[test]
        fn can_be_valid_map() {
            assert_de_tokens(&Color(1, 2, 3), &[
                T::Map{len: None},
                T::Str("r"), T::U8(1), T::Str("g"), T::U8(2), T::Str("b"), T::U8(3),
                T::MapEnd,
            ]);
            assert_de_tokens(&Color(1, 2, 3), &[
                T::Map{len: None},
                T::Str("red"), T::U8(1), T::Str("green"), T::U8(2), T::Str("blue"), T::U8(3),
                T::MapEnd,
            ]);
            // Mixed long/short field names are actually allowed...
            assert_de_tokens(&Color(1, 2, 3), &[
                T::Map{len: None},
                T::Str("r"), T::U8(1), T::Str("green"), T::U8(2), T::Str("b"), T::U8(3),
                T::MapEnd,
            ]);
        }

        #[test]
        fn cannot_be_invalid_map() {
            assert_de_tokens_error::<Color>(
                &[T::Map{len: Some(0)}], "invalid length 0, expected 3");
            assert_de_tokens_error::<Color>(
                &[T::Map{len: None}, T::MapEnd], "missing field `r`");
            assert_de_tokens_error::<Color>(
                &[T::Map{len: None}, T::Str("weird"), T::Str("wat")],
                &format!("unknown field `weird`, expected {}", *EXPECTING_FIELD_MSG));
            assert_de_tokens_error::<Color>(&[
                T::Map{len: None},
                T::Str("r"), T::U8(255),
                T::Str("b"), T::U8(0),
                T::MapEnd,
            ], "missing field `g`");
            assert_de_tokens_error::<Color>(
                &[T::Map{len: Some(5)}], "invalid length 5, expected 3");
        }
    }

    mod from_str {
        use std::str::FromStr;
        use spectral::prelude::*;
        use super::super::{Color, ColorParseError};

        #[test]
        fn pure_named_colors() {
            assert_that!(Color::from_str("black")).is_ok().is_equal_to(Color(0, 0, 0));
            assert_that!(Color::from_str("white")).is_ok().is_equal_to(Color(0xff, 0xff, 0xff));
            assert_that!(Color::from_str("red")).is_ok().is_equal_to(Color(0xff, 0, 0));
            assert_that!(Color::from_str("lime")).is_ok().is_equal_to(Color(0, 0xff, 0));  // "green" is just half green
            assert_that!(Color::from_str("blue")).is_ok().is_equal_to(Color(0, 0, 0xff));
        }

        #[test]
        fn common_named_colors() {
            assert_that!(Color::from_str("gray")).is_ok().is_equal_to(Color(0x80, 0x80, 0x80));
            assert_that!(Color::from_str("silver")).is_ok().is_equal_to(Color(192, 192, 192));
            assert_that!(Color::from_str("teal")).is_ok().is_equal_to(Color(0, 0x80, 0x80));
            assert_that!(Color::from_str("brown")).is_ok().is_equal_to(Color(165, 42, 42));
            assert_that!(Color::from_str("maroon")).is_ok().is_equal_to(Color(0x80, 0, 0));
            assert_that!(Color::from_str("navy")).is_ok().is_equal_to(Color(0, 0, 0x80));
            assert_that!(Color::from_str("green")).is_ok().is_equal_to(Color(0, 0x80, 0));
            assert_that!(Color::from_str("magenta")).is_ok().is_equal_to(Color(0xff, 0, 0xff));
            assert_that!(Color::from_str("cyan")).is_ok().is_equal_to(Color(0, 0xff, 0xff));
            assert_that!(Color::from_str("yellow")).is_ok().is_equal_to(Color(0xff, 0xff, 0));
        }

        #[test]
        fn exotic_named_colors() {
            assert_that!(Color::from_str("aquamarine")).is_ok().is_equal_to(Color(127, 255, 212));
            assert_that!(Color::from_str("bisque")).is_ok().is_equal_to(Color(255, 228, 196));
            assert_that!(Color::from_str("chocolate")).is_ok().is_equal_to(Color(210, 105, 30));
            assert_that!(Color::from_str("crimson")).is_ok().is_equal_to(Color(220, 20, 60));
            assert_that!(Color::from_str("darksalmon")).is_ok().is_equal_to(Color(233, 150, 122));
            assert_that!(Color::from_str("firebrick")).is_ok().is_equal_to(Color(178, 34, 34));
            assert_that!(Color::from_str("ivory")).is_ok().is_equal_to(Color(255, 255, 240));
            assert_that!(Color::from_str("lavender")).is_ok().is_equal_to(Color(230, 230, 250));
            assert_that!(Color::from_str("lightsteelblue")).is_ok().is_equal_to(Color(176, 196, 222));
            assert_that!(Color::from_str("mediumseagreen")).is_ok().is_equal_to(Color(60, 179, 113));
            assert_that!(Color::from_str("paleturquoise")).is_ok().is_equal_to(Color(175, 238, 238));
            assert_that!(Color::from_str("sienna")).is_ok().is_equal_to(Color(160, 82, 45));
            assert_that!(Color::from_str("tomato")).is_ok().is_equal_to(Color(255, 99, 71));
            assert_that!(Color::from_str("wheat")).is_ok().is_equal_to(Color(245, 222, 179));
            assert_that!(Color::from_str("yellowgreen")).is_ok().is_equal_to(Color(154, 205, 50));
            // ...and that's not even all of them!
        }

        #[test]
        fn html_rgb() {
            assert_that!(Color::from_str("#0f0")).is_ok().is_equal_to(Color(0, 0xff, 0));
            assert_that!(Color::from_str("#00ff00")).is_ok().is_equal_to(Color(0, 0xff, 0));
            assert_that!(Color::from_str("0xff0000")).is_ok().is_equal_to(Color(0xff, 0, 0));
            assert_that!(Color::from_str("$0000ff")).is_ok().is_equal_to(Color(0, 0, 0xff));
            // These are forbidden because it's unclear what they would mean.
            assert_that!(Color::from_str("0xf0f")).is_err();
            assert_that!(Color::from_str("$ff0")).is_err();
            // Multiple prefixes are NOT cleared.
            assert_that!(Color::from_str("$0x00ffff")).is_err();
            // We do need a prefix though (otherwise it's ambiguous if it's hex or name).
            assert_that!(Color::from_str("f0f0f0")).is_err();
        }

        #[test]
        fn transparency_not_supported() {
            assert_that!(Color::from_str("transparent"))
                .is_err().is_equal_to(ColorParseError::Alpha(0.0));
            assert_that!(Color::from_str("rgba(0, 0, 0, 0.5)"))
                .is_err().is_equal_to(ColorParseError::Alpha(0.5));
        }
    }
}
