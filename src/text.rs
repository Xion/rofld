//! Module responsible for rendering text.

use std::ops::{Add, Div, Sub};

use image::{DynamicImage, GenericImage, Rgba};
use num::One;
use rusttype::{Font, point, Point, Rect, Scale, Vector};


/// Vertical alignment of text within a rectangle.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Middle,
    Bottom,
}

/// Horizontal alignment of text within a rectangle.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HAlign {
    Left,
    Center,
    Right,
}


/// Alignment of text within a rectangle.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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


/// Renders a line of text onto given image.
pub fn render_line<A: Into<Alignment>>(img: DynamicImage,
                                       s: &str,
                                       align: A, offset: Vector<f32>,
                                       font: &Font, size: f32) -> DynamicImage {
    let mut img = img;
    let align: Alignment = align.into();
    trace!("render_line(..., {:?}, {:?}, offset={:?}, ..., size={})",
        s, align, offset, size);

    // Rendering text requires alpha blending.
    if img.as_rgba8().is_none() {
        img = DynamicImage::ImageRgba8(img.to_rgba());
    }

    let scale = Scale::uniform(size);
    let v_metrics = font.v_metrics(scale);

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
        // Compute width as the final X position of the "caret"
        // after laying out the glyphs starting from X=0.
        let glyphs: Vec<_> = font.layout(s, scale, point(0.0, /* unused */ 0.0)).collect();
        let width = glyphs.iter()
            .rev()
            .filter_map(|g| g.pixel_bounding_box().map(|bb| {
                bb.min.x as f32 + g.unpositioned().h_metrics().advance_width
            }))
            .next().unwrap_or(0.0);
        match align.horizontal {
            HAlign::Center => position.x -= width / 2.0,
            HAlign::Right => position.x -= width,
            _ => unreachable!(),
        }
    }
    match align.vertical {
        VAlign::Top => position.y += v_metrics.ascent,
        VAlign::Middle => {
            let height = size;
            position.y += v_metrics.ascent - height / 2.0;
        },
        VAlign::Bottom => {
            position.y -= v_metrics.descent.abs();  // it's usually negative
        },
    }

    // Now we can draw the text.
    // TODO: allow to specify text color (probably combine font, size, and color
    // into Style structure)
    for glyph in font.layout(s, scale, position) {
        if let Some(bbox) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                let x = (bbox.min.x + x as i32) as u32;
                let y = (bbox.min.y + y as i32) as u32;
                let alpha = (v * 255f32) as u8;
                if x < width && y < height {
                    img.blend_pixel(x, y, Rgba{data: [255, 255, 255, alpha]});
                }
            });
        }
    }

    img
}
