//! Deserializers for the alignment types.
//!
//! This is implemented manually rather than using #[derive(Deserialize)]
//! because serde_qs doesn't seem to work with the derived version,
//! producing silly errors such as:
//! ```notrust
//! Error { err: "invalid type: string \"top\", expected enum VAlign" }
//! ```

// TODO: delete this module and use #[derive(Deserialize)] when
// https://github.com/samscott89/serde_qs/issues/6 is fixed

use std::fmt;

use serde::de::{self, Deserialize, Unexpected, Visitor};

use super::super::{HAlign, VAlign};


impl<'de> Deserialize<'de> for HAlign {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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
