//! Module responsible for rendering text.

use std::fmt;
use std::ops::{Add, Div, Sub};

use image::{DynamicImage, GenericImage};
use num::One;
use rusttype::{Font, point, Point, Rect, Scale, Vector};
use unreachable::unreachable;

use model::{Color, HAlign, VAlign};


/// Alignment of text within a rectangle.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Alignment {
    pub vertical: VAlign,
    pub horizontal: HAlign,
}

impl Alignment {
    #[inline]
    pub fn new(vertical: VAlign, horizontal: HAlign) -> Self {
        Alignment{vertical: vertical, horizontal: horizontal}
    }
}

impl From<(VAlign, HAlign)> for Alignment {
    fn from((v, h): (VAlign, HAlign)) -> Self {
        Alignment::new(v, h)
    }
}
impl From<(HAlign, VAlign)> for Alignment {
    fn from((h, v): (HAlign, VAlign)) -> Self {
        Alignment::new(v, h)
    }
}

impl fmt::Debug for Alignment {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Alignment::{:?}{:?}", self.vertical, self.horizontal)
    }
}

impl Alignment {
    /// The origin point for this alignment within given rectangle.
    /// Returns one of nine possible points at the edges of the rectangle.
    pub fn origin_within<N>(&self, rect: Rect<N>) -> Point<N>
        where N: Copy + One + Add<Output=N> + Sub<Output=N> + Div<Output=N>
    {
        let two = N::one() + N::one();
        let x = match self.horizontal {
            HAlign::Left => rect.min.x,
            HAlign::Center => rect.min.x + rect.width() / two,
            HAlign::Right => rect.max.x,
        };
        let y = match self.vertical {
            VAlign::Top => rect.min.y,
            VAlign::Middle => rect.min.y + rect.height() / two,
            VAlign::Bottom => rect.max.y,
        };
        point(x, y)
    }
}


/// Style that the text is rendered with.
pub struct Style<'f> {
    font: &'f Font<'f>,
    size: f32,
    color: Color,
}

impl<'f> Style<'f> {
    #[inline]
    pub fn new(font: &'f Font, size: f32, color: Color) -> Self {
        if size <= 0.0 {
            panic!("text::Style got negative size ({})", size);
        }
        Style{font: font, size: size, color: color}
    }

    #[inline]
    pub fn scale(&self) -> Scale {
        Scale::uniform(self.size)
    }
}

impl<'f> fmt::Debug for Style<'f> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Style")
            .field("font", &"Font{}")  // we don't have any displayable info here
            .field("size", &self.size)
            .field("color", &self.color)
            .finish()
    }
}


/// Renders text onto given image.
pub fn render_text<A: Into<Alignment>>(img: DynamicImage,
                                       s: &str,
                                       align: A, offset: Vector<f32>,
                                       style: Style) -> DynamicImage {
    let mut img = img;
    let align: Alignment = align.into();
    trace!("render_text(..., <length: {}>, {:?}, offset={:?}, {:?})",
        s.len(), align, offset, style);

    let line_width = img.width() as f32 - offset.x;
    let lines = break_lines(s, &style, line_width);
    trace!("Text broken into {} line(s)", lines.len());

    let v_metrics = style.font.v_metrics(style.scale());
    let line_height = v_metrics.ascent.abs() +
                      v_metrics.descent.abs() +
                      v_metrics.line_gap;

    let mut offset = offset;
    for line in lines {
        img = render_line(img, &line, align, offset, &style);
        offset.y += line_height;
    }
    img
}


/// Renders a line of text onto given image.
///
/// Text should be single-line (line breaks are ignored)
/// and short enough to fit (or it will be clipped).
pub fn render_line<A: Into<Alignment>>(img: DynamicImage,
                                       s: &str,
                                       align: A, offset: Vector<f32>,
                                       style: &Style) -> DynamicImage {
    let mut img = img;
    let align: Alignment = align.into();
    trace!("render_line(..., {:?}, {:?}, offset={:?}, {:?})",
        s, align, offset, style);

    // Rendering text requires alpha blending.
    if img.as_rgba8().is_none() {
        img = DynamicImage::ImageRgba8(img.to_rgba());
    }

    let scale = style.scale();
    let v_metrics = style.font.v_metrics(scale);

    // Figure out where we're drawing.
    //
    // Unless it's a straightforward rendering in the top-left corner,
    // we need to compute the final bounds of the text first,
    // so that we can account for it when computing the start position.
    //
    let (x, y, width, height) = img.bounds();
    let image_rect: Rect<f32> = Rect{
        min: point(x as f32, y as f32),
        max: point((x + width) as f32, (y + height) as f32),
    };
    let mut position = align.origin_within(image_rect) + offset;
    if align.horizontal != HAlign::Left {
        let width = text_width(s, &style);
        match align.horizontal {
            HAlign::Center => position.x -= width / 2.0,
            HAlign::Right => position.x -= width,
            _ => unsafe { unreachable(); },
        }
    }
    match align.vertical {
        VAlign::Top => position.y += v_metrics.ascent,
        VAlign::Middle => {
            let height = style.size;
            position.y += v_metrics.ascent - height / 2.0;
        },
        VAlign::Bottom => {
            position.y -= v_metrics.descent.abs();  // it's usually negative
        },
    }

    // Now we can draw the text.
    for glyph in style.font.layout(s, scale, position) {
        if let Some(bbox) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                let x = (bbox.min.x + x as i32) as u32;
                let y = (bbox.min.y + y as i32) as u32;
                let alpha = (v * 255f32) as u8;
                if x < width && y < height {
                    img.blend_pixel(x, y, style.color.to_rgba(alpha));
                }
            });
        }
    }

    img
}


// Utility functions

/// Break the text into lines, fitting given width.
fn break_lines(s: &str, style: &Style, line_width: f32) -> Vec<String> {
    // XXX: honor explicit line breaks
    let words: Vec<&str> = s.split(|c: char| c.is_whitespace()).collect();
    trace!("Computing line breaks for text of length {} with {} word(s)",
        s.len(), words.len());

    // TODO: handle different kinds of whitespace that may be separating words
    let space_width = text_width(" ", style);

    let mut result = vec![];

    let mut current_line = String::new();
    let mut current_width = 0.0;
    for word in words {
        let word_width = text_width(word, style);
        if current_width + word_width + space_width > line_width {
            // TODO: if the word itself is too long, break it wherever to fit
            if !current_line.is_empty() {
                result.push(current_line.clone());
                current_line.clear();
            }
            current_width = 0.0;
        }
        current_line.push_str(word);
        current_line.push(' ');
        current_width += word_width + space_width;
    }
    if !current_line.is_empty() {
        result.push(current_line);
    }

    result
}

/// Compute the pixel width of given text.
fn text_width(s: &str, style: &Style) -> f32 {
    // Compute text width as the final X position of the "caret"
    // after laying out all the glyphs, starting from X=0.
    let glyphs: Vec<_> = style.font
        .layout(s, style.scale(), point(0.0, /* unused */ 0.0))
        .collect();
    glyphs.iter()
        .rev()
        .filter_map(|g| g.pixel_bounding_box().map(|bb| {
            bb.min.x as f32 + g.unpositioned().h_metrics().advance_width
        }))
        .next().unwrap_or(0.0)
}
