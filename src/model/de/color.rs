//! Deserializer for the Color type.

use std::fmt;
use std::ops::Range;

use serde::de::{self, Deserialize, Unexpected, Visitor};

use super::super::Color;


const FIELDS: &'static [&'static str] = &["r", "g", "b"];


impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: de::Deserializer<'de>
    {
        deserializer.deserialize_tuple_struct("Color", 3, ColorVisitor)
    }
}

struct ColorVisitor;
impl<'de> Visitor<'de> for ColorVisitor {
    type Value = Color;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "HTML color string or array/map of RGB values")
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
        let mut s = s.trim();
        for prefix in ["#", "0x", "$"].into_iter() {
            s = s.trim_left_matches(prefix);
        }
        let mut s = s.to_lowercase();

        // TODO: support "rgb($r, $g, $b)" like CSS does
        // TODO: support canned list of CSS colors

        // rgb -> rrggbb
        if s.len() == 3 {
            s = format!("{r}{r}{g}{g}{b}{b}", r=&s[0..1], g=&s[1..2], b=&s[2..3]);
        }

        let channel = |range: Range<usize>| {
            u8::from_str_radix(&s[range.clone()], 16).map_err(|_| {
                E::invalid_value(Unexpected::Str(&s[range]), &"8-bit hexadecimal number")
            })
        };
        let r = channel(0..2)?;
        let g = channel(2..4)?;
        let b = channel(4..6)?;
        Ok(Color(r, g, b))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where A: de::SeqAccess<'de>
    {
        let mut channels = Vec::with_capacity(3);
        while let Some(elem) = seq.next_element::<u8>()? {
            channels.push(elem);
        }

        if channels.len() != 3 {
            return Err(de::Error::invalid_length(channels.len(), &self));
        }
        let mut result = channels.into_iter();
        Ok(Color(result.next().unwrap(),
                 result.next().unwrap(),
                 result.next().unwrap()))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where A: de::MapAccess<'de>
    {
        let (mut r, mut g, mut b) = (None, None, None);
        while let Some(key) = map.next_key::<String>()? {
            let key = key.trim().to_lowercase();
            match key.as_str() {
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
