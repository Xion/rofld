//! Module implementing the actual image captioning.

mod cache;
pub mod fonts;
pub mod templates;


use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::sync::Arc;

use futures::{Future, future};
use futures_cpupool::{self, CpuPool};
use hyper::StatusCode;
use hyper::server::Response;
use image::{self, DynamicImage, FilterType, GenericImage};
use rusttype::vector;

use text::{self, HAlign, VAlign, Style};
use util::error_response;
use self::cache::Cache;


/// Describes an image macro, used as an input structure.
#[derive(Deserialize)]
pub struct ImageMacro {
    template: String,
    width: Option<u32>,
    height: Option<u32>,

    font: Option<String>,
    top_text: Option<String>,
    middle_text: Option<String>,
    bottom_text: Option<String>,
}

impl ImageMacro {
    #[inline]
    pub fn has_text(&self) -> bool {
        self.top_text.is_some() || self.middle_text.is_some() || self.bottom_text.is_some()
    }
}

impl fmt::Debug for ImageMacro {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut ds = fmt.debug_struct("ImageMacro");
        ds.field("template", &self.template);

        if let Some(ref width) = self.width {
            ds.field("width", width);
        }
        if let Some(ref height) = self.height {
            ds.field("height", height);
        }

        if let Some(ref font) = self.font {
            ds.field("font", font);
        }
        if let Some(ref text) = self.top_text {
            ds.field("top_text", text);
        }
        if let Some(ref text) = self.middle_text {
            ds.field("middle_text", text);
        }
        if let Some(ref text) = self.bottom_text {
            ds.field("bottom_text", text);
        }

        ds.finish()
    }
}


/// Type encapsulating all the image captioning logic.
pub struct Captioner {
    pool: CpuPool,
    cache: Arc<Cache>,
}

impl Captioner {
    #[inline]
    fn new( ) -> Self {
        // TODO: configure the number of threads from the command line
        let pool = futures_cpupool::Builder::new()
            .name_prefix("caption-")
            .create();
        let cache = Arc::new(Cache::new());
        Captioner{pool: pool, cache: cache}
    }
}

// Rendering code.
impl Captioner {
    /// Render an image macro as PNG into the specified Writer.
    /// The rendering is done in a separate thread.
    pub fn render(&self, im: ImageMacro) -> Box<Future<Item=Vec<u8>, Error=CaptionError>> {
        self.pool.clone().spawn_fn({
            // TODO: encapsulate the state of the rendering process in a thread-local
            // structure (containing e.g. the Cache reference)
            let cache = self.cache.clone();
            move || {
                let mut image_bytes = vec![];
                match Self::do_render(cache, &im, &mut image_bytes) {
                    Ok(_) => {
                        debug!("Successfully rendered {:?}", im);
                        future::ok(image_bytes)
                    },
                    Err(e) => {
                        error!("Failed to render image macro {:?}: {}", im, e);
                        future::err(e)
                    },
                }
            }
        })
        .boxed()
    }

    fn do_render<W: Write>(cache: Arc<Cache>, im: &ImageMacro, writer: &mut W) -> Result<(), CaptionError> {
        debug!("Rendering {:?}", im);

        let template = cache.get_template(&im.template)
            .ok_or_else(|| CaptionError::Template(im.template.clone()))?;

        // Resize the image to fit within the given dimensions.
        // Note that the resizing preserves original aspect, so the final image
        // may be smaller than requested.
        let (orig_width, orig_height) = template.dimensions();
        trace!("Original size of the template `{}`: {}x{}",
            im.template, orig_width, orig_height);
        let target_width = im.width.unwrap_or(orig_width);
        let target_height = im.height.unwrap_or(orig_height);
        let mut img;
        if target_width != orig_width || target_height != orig_height {
            debug!("Resizing template `{}` from {}x{} to {}x{}",
                im.template, orig_width, orig_height, target_width, target_height);
            img = template.resize(target_width, target_height, FilterType::Lanczos3);
        } else {
            debug!("Using original template size of {}x{}", orig_width, orig_height);
            img = (*template).clone();  // clone the actual image
        }
        let (width, height) = img.dimensions();
        trace!("Final image size: {}x{}", width, height);

        if im.has_text() {
            img = Self::draw_text(cache, im, img)?;
        }

        debug!("Encoding final image as PNG...");
        image::png::PNGEncoder::new(writer)
            .encode(&*img.raw_pixels(), width, height, img.color())
            .map_err(CaptionError::Encode)
    }

    fn draw_text(cache: Arc<Cache>, im: &ImageMacro, img: DynamicImage) -> Result<DynamicImage, CaptionError> {
        // Rendering text requires alpha blending.
        let mut img = img;
        if img.as_rgba8().is_none() {
            trace!("Converting image to RGBA...");
            img = DynamicImage::ImageRgba8(img.to_rgba());
        }

        let font_name = im.font.as_ref().map(|s| s.as_str()).unwrap_or(fonts::DEFAULT);
        let font = cache.get_font(font_name)
            .ok_or_else(|| CaptionError::Font(font_name.to_owned()))?;

        // TODO: moar constants, better encapsulation, all that jazz
        let size = 64.0;
        if let Some(ref top_text) = im.top_text {
            let alignment = (VAlign::Top, HAlign::Center);
            let top_margin_px = 16.0;
            debug!("Rendering top text: {}", top_text);
            img = text::render_line(
                img, top_text, alignment, vector(0.0, top_margin_px),
                Style::white(&font, size));
        }
        if let Some(ref middle_text) = im.middle_text {
            let alignment = (VAlign::Middle, HAlign::Center);
            debug!("Rendering middle text: {}", middle_text);
            img = text::render_line(
                img, middle_text, alignment, vector(0.0, 0.0),
                Style::white(&font, size));
        }
        if let Some(ref bottom_text) = im.bottom_text {
            let alignment = (VAlign::Bottom, HAlign::Center);
            let bottom_margin_px =  16.0;
            debug!("Rendering bottom text: {}", bottom_text);
            img = text::render_line(
                img, bottom_text, alignment, vector(0.0, -bottom_margin_px),
                Style::white(&font, size));
        }

        Ok(img)
    }
}

lazy_static! {
    /// The singleton instance of Captioner.
    /// This is done to share the caches it holds.
    pub static ref CAPTIONER: Arc<Captioner> = Arc::new(Captioner::new());
}


/// Error that may occur during the captioning.
#[derive(Debug)]
pub enum CaptionError {
    Template(String),
    Font(String),
    Encode(io::Error),
}
unsafe impl Send for CaptionError {}

impl CaptionError {
    #[inline]
    pub fn status_code(&self) -> StatusCode {
        match *self {
            CaptionError::Template(..) => StatusCode::NotFound,
            CaptionError::Font(..) => StatusCode::NotFound,
            CaptionError::Encode(..) => StatusCode::InternalServerError,
        }
    }
}

impl Error for CaptionError {
    fn description(&self) -> &str { "captioning error" }
    fn cause(&self) -> Option<&Error> {
        match *self {
            CaptionError::Encode(ref e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for CaptionError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CaptionError::Template(ref t) => write!(fmt, "cannot find template `{}`", t),
            CaptionError::Font(ref f) => write!(fmt, "cannot find font `{}`", f),
            CaptionError::Encode(ref e) => write!(fmt, "failed to encode the  final image: {}", e),
        }
    }
}

impl Into<Response> for CaptionError {
    fn into(self) -> Response {
        error_response(self.status_code(), format!("{}", self))
    }
}
