//! Deserializer for the ImageMacro type.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::mem;

use itertools::Itertools;
use serde::de::{self, Deserialize, IntoDeserializer, Visitor, Unexpected};
use unicode_normalization::UnicodeNormalization;
use unreachable::unreachable;

use super::super::{Caption, Color, ImageMacro, Size, VAlign,
                   DEFAULT_COLOR, DEFAULT_OUTLINE_COLOR, DEFAULT_FONT, DEFAULT_HALIGN,
                   MAX_CAPTION_COUNT, MAX_WIDTH, MAX_HEIGHT, MAX_CAPTION_LENGTH};


/// Publicly mentioned fields of ImageMacro.
const FIELDS: &'static [&'static str] = &[
    "template", "width", "height", "captions",
];
/// Semi-official fields that allow to set properties of all captions at once.
const WHOLESALE_CAPTION_FIELDS: &'static [&'static str] = &[
    "font", "color", "outline", "size",
];
// How many fields (of any kind) are required at the very minimum.
const REQUIRED_FIELDS_COUNT: usize = 1;  // template

const EXPECTING_MSG: &'static str = "representation of an image macro";
lazy_static! {
    static ref EXPECTING_FIELD_COUNT_MSG: String = format!("at least {}", REQUIRED_FIELDS_COUNT);
}


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
        write!(fmt,"{}", EXPECTING_MSG)
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
        where V: de::MapAccess<'de>
    {
        // Preemptively check for length against a minimum.
        if let Some(size) = map.size_hint() {
            if size < REQUIRED_FIELDS_COUNT {
                return Err(de::Error::invalid_length(
                    size, &(&*EXPECTING_FIELD_COUNT_MSG as &str)));
            }
        }

        let mut template = None;
        let mut width = None;
        let mut height = None;

        let mut simple_fields = HashSet::new();
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
                    let value: String = map.next_value()?;
                    trace!("ImageMacro::template = {}", value);
                    if value.is_empty() {
                        return Err(de::Error::invalid_value(
                            Unexpected::Str(&value), &"non-empty string"));
                    }
                    template = Some(value);
                }
                "width" => {
                    if width.is_some() {
                        return Err(de::Error::duplicate_field("width"));
                    }
                    let value = map.next_value()?;
                    trace!("ImageMacro::width = {}", value);
                    if value > MAX_WIDTH {
                        return Err(de::Error::custom(
                            format_args!("width is too large: {} > {}", value, MAX_WIDTH)));
                    }
                    width = Some(value);
                }
                "height" => {
                    if height.is_some() {
                        return Err(de::Error::duplicate_field("height"));
                    }
                    let value = map.next_value()?;
                    trace!("ImageMacro::height = {}", value);
                    if value > MAX_HEIGHT {
                        return Err(de::Error::custom(
                            format_args!("height is too large: {} > {}", value, MAX_HEIGHT)));
                    }
                    height = Some(value);
                }

                // Simplified way of defining top/middle/bottom captions.
                "top_text"    | "middle_text"    | "bottom_text"    |
                "top_align"   | "middle_align"   | "bottom_align"   |
                "top_font"    | "middle_font"    | "bottom_font"    |
                "top_color"   | "middle_color"   | "bottom_color"   |
                "top_outline" | "middle_outline" | "bottom_outline" |
                "top_size"    | "middle_size"    | "bottom_size"    => {
                    let is_duplicate = simple_fields.contains(&key) ||
                        WHOLESALE_CAPTION_FIELDS.iter().any(|&f| {
                            key.ends_with(&format!("_{}", f)) && simple_fields.contains(f)
                        });
                    if is_duplicate {
                        return Err(de::Error::custom(format_args!("duplicate field `{}`", key)));
                    }
                    simple_fields.insert(key.clone());
                    trace!("ImageMacro::{} = <snip>", key);

                    let mut parts = key.split("_");
                    let (valign_part, field_part) = (parts.next().unwrap(),
                                                     parts.next().unwrap());

                    // Put the horizontal align / text into the correct Caption.
                    let valign_de =
                        IntoDeserializer::<de::value::Error>::into_deserializer(valign_part);
                    let valign = VAlign::deserialize(valign_de).unwrap();
                    let caption = simple_captions.entry(valign)
                        .or_insert_with(|| Caption::at(valign));

                    match field_part {
                        "text" => caption.text = map.next_value()?,
                        "align" => caption.halign = map.next_value()?,
                        "font" => caption.font = map.next_value()?,
                        "color" => caption.color = map.next_value()?,
                        "outline" => caption.outline = map.next_value()?,
                        "size" => caption.size = map.next_value()?,
                        _ => unsafe { unreachable(); },
                    }
                }

                // Wholesale setting of simple captions' properties.
                "font" => {
                    assert!(WHOLESALE_CAPTION_FIELDS.contains(&key.as_str()));

                    let is_duplicate = simple_fields.contains(&key) ||
                        simple_fields.iter().any(|f| f.ends_with("_font"));
                    if is_duplicate {
                        return Err(de::Error::duplicate_field("font"));
                    }
                    simple_fields.insert("font".into());

                    let font: String = map.next_value()?;
                    trace!("ImageMacro::font = {}", font);
                    for valign in VAlign::iter_variants() {
                        let caption = simple_captions.entry(valign)
                            .or_insert_with(|| Caption::at(valign));
                        caption.font = font.clone();
                    }
                }
                "color" => {
                    assert!(WHOLESALE_CAPTION_FIELDS.contains(&key.as_str()));

                    let is_duplicate = simple_fields.contains(&key) ||
                        simple_fields.iter().any(|f| f.ends_with("_color"));
                    if is_duplicate {
                        return Err(de::Error::duplicate_field("color"));
                    }
                    simple_fields.insert("color".into());

                    let color: Color = map.next_value()?;
                    trace!("ImageMacro::color = {}", color);
                    for valign in VAlign::iter_variants() {
                        let caption = simple_captions.entry(valign)
                            .or_insert_with(|| Caption::at(valign));
                        caption.color = color;
                    }
                }
                "outline" => {
                    assert!(WHOLESALE_CAPTION_FIELDS.contains(&key.as_str()));

                    let is_duplicate = simple_fields.contains(&key) ||
                        simple_fields.iter().any(|f| f.ends_with("_outline"));
                    if is_duplicate {
                        return Err(de::Error::duplicate_field("outline"));
                    }
                    simple_fields.insert("outline".into());

                    let outline: Option<Color> = map.next_value()?;
                    trace!("ImageMacro::outline = {:?}", outline);
                    for valign in VAlign::iter_variants() {
                        let caption = simple_captions.entry(valign)
                            .or_insert_with(|| Caption::at(valign));
                        caption.outline = outline;
                    }
                }
                "size" => {
                    assert!(WHOLESALE_CAPTION_FIELDS.contains(&key.as_str()));

                    let is_duplicate = simple_fields.contains(&key) ||
                        simple_fields.iter().any(|f| f.ends_with("_size"));
                    if is_duplicate {
                        return Err(de::Error::duplicate_field("size"));
                    }
                    simple_fields.insert("size".into());

                    let size: Size = map.next_value()?;
                    trace!("ImageMacro::size = {:?}", size);
                    for valign in VAlign::iter_variants() {
                        let caption = simple_captions.entry(valign)
                            .or_insert_with(|| Caption::at(valign));
                        caption.size = size;
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
                            .map(From::from).collect();
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
                        let captions: Vec<_> = captions.collect();
                        if captions.len() > MAX_CAPTION_COUNT {
                            return Err(de::Error::custom(
                                format_args!("there can be at most {} captions", MAX_CAPTION_COUNT)));
                        }
                        full_captions = Some(captions);
                    }
                }

                key => return Err(de::Error::unknown_field(key, FIELDS)),
            }
        }

        // The input should either use the full "captions" field,
        // or the simpler version with top/middle/bottom_test/align -- but not both.
        let mut captions;
        if full_captions.is_some() {
            if simple_captions.len() > 0 {
                return Err(de::Error::custom(
                    "`captions` cannot be provided along with `top/middle/bottom_text/align/etc.`"))
            }
            if simple_fields.len() > 0 {
                return Err(de::Error::custom(
                    format_args!("custom `{}` cannot be provided along with `captions`",
                        simple_fields.iter().next().unwrap())));
            }
        }

        // Convert everything to "full" captions either way.
        if simple_captions.len() > 0 {
            captions = simple_captions.into_iter()
                .map(|(_, c)| c).filter(|c| !c.text.is_empty())
                .collect()
        } else {
            captions = full_captions.unwrap_or_else(|| vec![]);
        }
        captions.sort_by_key(|c| (c.valign, c.halign));
        for caption in &mut captions {
            caption.text = normalize_text(&caption.text)?;
        }

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

impl From<SourcedCaption> for (CaptionSource, Caption) {
    fn from(sc: SourcedCaption) -> Self {
        (sc.0, sc.1)
    }
}

impl<'de> Deserialize<'de> for SourcedCaption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: de::Deserializer<'de>
    {
        deserializer.deserialize_any(SourcedCaptionVisitor)
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
            size: Size::default(),
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


/// Normalize the caption text and validate it.
fn normalize_text<E: de::Error>(s: &str) -> Result<String, E> {
    // Use the NFC form as suggested by rusttype crate docs.
    let normalized = s.nfc();

    let mut count = 0;
    let text: String = normalized.map(|c| { count += 1; c }).collect();
    trace!("Caption text normalized to {} characters", count);
    if count > MAX_CAPTION_LENGTH {
        return Err(E::custom(format_args!(
            "caption text is too long: {} > {} characters", count, MAX_CAPTION_LENGTH)));
    }
    Ok(text)
}
