//! Deserializer for the ImageMacro type.

use std::collections::HashMap;
use std::fmt;
use std::mem;

use itertools::Itertools;
use serde::de::{self, Deserialize, IntoDeserializer, Visitor};
use unreachable::unreachable;

use super::super::{Caption, ImageMacro, VAlign,
                   DEFAULT_COLOR, DEFAULT_OUTLINE_COLOR, DEFAULT_FONT, DEFAULT_HALIGN};


const FIELDS: &'static [&'static str] = &[
    "template", "width", "height", "font", "captions",
];


impl<'de> Deserialize<'de> for ImageMacro {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
        where V: de::MapAccess<'de>
    {
        let mut template = None;
        let mut width = None;
        let mut height = None;

        let mut simple_captions: HashMap<VAlign, Caption> = HashMap::new();
        let mut full_captions: Option<Vec<Caption>> = None;

        while let Some(key) = map.next_key::<String>()? {
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

                // Simplified way of defining top/middle/bottom captions.
                "top_text"    | "middle_text"    | "bottom_text"    |
                "top_align"   | "middle_align"   | "bottom_align"   |
                "top_font"    | "middle_font"    | "bottom_font"    |
                "top_color"   | "middle_color"   | "bottom_color"   |
                "top_outline" | "middle_outline" | "bottom_outline" => {
                    let mut parts = key.split("_");
                    let (valign_part, field_part) = (parts.next().unwrap(),
                                                     parts.next().unwrap());

                    // Put the horizontal align / text into the correct Caption.
                    let valign_de =
                        IntoDeserializer::<de::value::Error>::into_deserializer(valign_part);
                    let valign = VAlign::deserialize(valign_de).unwrap();
                    let mut caption = simple_captions.entry(valign)
                        .or_insert_with(|| Caption::at(valign));

                    match field_part {
                        "text" => caption.text = map.next_value()?,
                        "align" => caption.halign = map.next_value()?,
                        "font" => caption.font = map.next_value()?,
                        "color" => caption.color = map.next_value()?,
                        "outline" => caption.outline = map.next_value()?,
                        _ => unsafe { unreachable(); },
                    }
                }

                // Fully featured caption definition.
                "captions" => {
                    if full_captions.is_some() {
                        return Err(de::Error::duplicate_field("captions"));
                    }

                    // Deserialize captions, remembering which kind of input source they came from.
                    let sourced_captions: Vec<(CaptionSource, Caption)> =
                        map.next_value::<Vec<SourcedCaption>>()?.into_iter()
                            .map(|sc| (sc.0, sc.1)).collect();
                    if sourced_captions.iter().map(|&(s, _)| s).unique().count() > 1 {
                        return Err(de::Error::custom(
                            "captions must be either all texts, or all complete representations"));
                    }

                    if sourced_captions.is_empty() {
                        full_captions = Some(vec![]);
                        continue;
                    }
                    let source = sourced_captions[0].0;
                    let count = sourced_captions.len();
                    let captions = sourced_captions.into_iter().map(|(_, c)| c);

                    if source == CaptionSource::Text {
                        // Captions can be just text strings, laid down in the center,
                        // from top to bottom (depending on how many were provided).
                        let valigns = match count {
                            0 => vec![],
                            1 => vec![VAlign::Bottom],
                            2 => vec![VAlign::Top, VAlign::Bottom],
                            3 => vec![VAlign::Top, VAlign::Middle, VAlign::Bottom],
                            len => return Err(de::Error::invalid_length(len, &"0, 1, 2, or 3 strings")),
                        };
                        full_captions = Some(valigns.into_iter().zip(captions)
                            .map(|(v, c)| Caption { valign: v, ..c })
                            .collect());
                    } else {
                        full_captions = Some(captions.collect());
                    }
                }

                key => return Err(de::Error::unknown_field(key, FIELDS)),
            }
        }

        // The input should either use the full "captions" field,
        // or the simpler version with top/middle/bottom_test/align -- but not both.
        let mut captions;
        if simple_captions.len() > 0 && full_captions.is_some() {
            return Err(de::Error::custom(
                "`captions` cannot be provided along with `top/middle/bottom_text/align/font`"))
        }
        if simple_captions.len() > 0 {
            captions = simple_captions.into_iter().map(|(_, c)| c).collect()
        } else {
            captions = full_captions.unwrap_or_else(|| vec![]);
        }
        captions.sort_by_key(|c| (c.valign, c.halign));

        let template = template.ok_or_else(|| de::Error::missing_field("template"))?;
        Ok(ImageMacro{template, width, height, captions})
    }
}

// ImageMacro::captions can be provided as either a list of strings,
// or a list of complete Caption representations (as maps).
// However, we need to remember where the Captions originally came from
// in order to possibly pick the correct vertical alignment for them.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum CaptionSource { Text, Map }
struct SourcedCaption(CaptionSource, Caption);

impl<'de> Deserialize<'de> for SourcedCaption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: de::Deserializer<'de>
    {
        deserializer.deserialize_tuple_struct(
            "SourcedCaption", 2, SourcedCaptionVisitor)
    }
}

struct SourcedCaptionVisitor;
impl<'de> Visitor<'de> for SourcedCaptionVisitor {
    type Value = SourcedCaption;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "text or image macro's caption representation")
    }

    /// Deserialize Caption from a string.
    ///
    /// The vertical align of such caption is intentionally left undefined,
    /// as it should be filled in by the ImageMacro deserializer.
    fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
        let caption = Caption{
            text: text.to_owned(),
            halign: DEFAULT_HALIGN,
            valign: unsafe { mem::uninitialized() },
            font: DEFAULT_FONT.into(),
            color: DEFAULT_COLOR,
            outline: Some(DEFAULT_OUTLINE_COLOR),
        };
        let result = SourcedCaption(CaptionSource::Text, caption);
        Ok(result)
    }

    fn visit_map<V>(self, map: V) -> Result<Self::Value, V::Error>
        where V: de::MapAccess<'de>
    {
        // Use the default way of deserializing Caption from a map.
        let inner_de = de::value::MapAccessDeserializer::new(map);
        let caption = Deserialize::deserialize(inner_de)?;

        let result = SourcedCaption(CaptionSource::Map, caption);
        Ok(result)
    }
}
