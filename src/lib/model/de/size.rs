//! Deserializer for the Size type.

use std::fmt;
use std::str::FromStr;

use conv::errors::Unrepresentable;
use serde::de::{self, Deserialize, Unexpected, Visitor};

use super::super::Size;


const EXPECTING_MSG: &'static str = "numeric size, or \"shrink\"";


impl<'de> Deserialize<'de> for Size {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: de::Deserializer<'de>
    {
        deserializer.deserialize_any(SizeVisitor)
    }
}

struct SizeVisitor;
impl<'de> Visitor<'de> for SizeVisitor {
    type Value = Size;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", EXPECTING_MSG)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        let size = Size::from_str(v).map_err(|_| {
            warn!("Failed to parse size `{}`", v);
            E::invalid_value(Unexpected::Str(v), &self)
        })?;
        Ok(size)
    }

    fn visit_f32<E: de::Error>(self, v: f32) -> Result<Self::Value, E> {
        Ok(Size::from(v))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        let v32 = v as f32;
        if (v32 as f64) < v {
            warn!("Clamping the size float value from {} (64-bit) to {} (32-bit)",
                v, v32);
        }
        Ok(Size::from(v32))
    }

    // Other numeric visitor methods that delegate to the ones above.
    fn visit_i8<E: de::Error>(self, v: i8) -> Result<Self::Value, E> {
        self.visit_f32(v as f32)
    }
    fn visit_i16<E: de::Error>(self, v: i16) -> Result<Self::Value, E> {
        self.visit_f32(v as f32)
    }
    fn visit_i32<E: de::Error>(self, v: i32) -> Result<Self::Value, E> {
        self.visit_f32(v as f32)
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        self.visit_f64(v as f64)
    }
    fn visit_u8<E: de::Error>(self, v: u8) -> Result<Self::Value, E> {
        self.visit_f32(v as f32)
    }
    fn visit_u16<E: de::Error>(self, v: u16) -> Result<Self::Value, E> {
        self.visit_f32(v as f32)
    }
    fn visit_u32<E: de::Error>(self, v: u32) -> Result<Self::Value, E> {
        self.visit_f32(v as f32)
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        self.visit_f64(v as f64)
    }
}


impl FromStr for Size {
    type Err = Unrepresentable<String>;

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        match v.trim().to_lowercase().as_str() {
            "shrink" | "fit" | "flex" => Ok(Size::Shrink),
            // We can allow stringified numbers too,
            // just don't have it mentioned anywhere :)
            s => s.parse::<f32>().map(Into::into)
                .map_err(|_| Unrepresentable(v.to_owned())),
        }
    }
}


// TODO: tests, like in super::color
