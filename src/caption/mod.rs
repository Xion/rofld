//! Module implementing the actual image captioning.

mod cache;
pub mod fonts;
pub mod templates;


use std::error::Error;
use std::fmt;
use std::io;
use std::ops::Deref;
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

    #[inline]
    pub fn font(&self) -> &str {
        self.font.as_ref().map(|s| s.as_str()).unwrap_or(fonts::DEFAULT)
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


/// Renders image macros into captioned images in separate threads.
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
    /// Render an image macro as PNG.
    /// The rendering is done in a separate thread.
    pub fn render(&self, im: ImageMacro) -> Box<Future<Item=Vec<u8>, Error=CaptionError>> {
        self.pool.clone().spawn_fn({
            let im_repr = format!("{:?}", im);
            let task = CaptionTask{
                image_macro: im,
                cache: self.cache.clone(),
            };
            move || {
                match task.perform() {
                    Ok(ib) => {
                        debug!("Successfully rendered {}, final image size: {} bytes",
                            im_repr, ib.len());
                        future::ok(ib)
                    },
                    Err(e) => {
                        error!("Failed to render image macro {}: {}", im_repr, e);
                        future::err(e)
                    },
                }
            }
        })
        .boxed()
    }
}

lazy_static! {
    /// The singleton instance of Captioner.
    /// This is done to share the caches it holds.
    pub static ref CAPTIONER: Arc<Captioner> = Arc::new(Captioner::new());
}

/// Represents a single captioning task and contains all the relevant logic.
///
/// This is a separate struct so that its methods can be conveniently executed
/// in a separate thread.
struct CaptionTask {
    image_macro: ImageMacro,
    cache: Arc<Cache>,
}

impl Deref for CaptionTask {
    type Target = ImageMacro;
    fn deref(&self) -> &Self::Target {
        &self.image_macro  // makes the rendering code a little terser
    }
}

impl CaptionTask {
    /// Perform the captioning task.
    fn perform(self) -> Result<Vec<u8>, CaptionError> {
        debug!("Rendering {:?}", self.image_macro);

        let template = self.cache.get_template(&self.template)
            .ok_or_else(|| CaptionError::Template(self.template.clone()))?;

        let mut img = self.resize_template(template);
        if self.has_text() {
            img = self.draw_text(img)?;
        }
        self.encode_image(img)
    }

    /// Resize the template image to fit the desired dimensions.
    fn resize_template(&self, template: Arc<DynamicImage>) -> DynamicImage {
        // Note that the resizing preserves original aspect, so the final image
        // may be smaller than requested.
        let (orig_width, orig_height) = template.dimensions();
        trace!("Original size of the template `{}`: {}x{}",
            self.template, orig_width, orig_height);
        let target_width = self.width.unwrap_or(orig_width);
        let target_height = self.height.unwrap_or(orig_height);

        let img;
        if target_width != orig_width || target_height != orig_height {
            debug!("Resizing template `{}` from {}x{} to {}x{}",
                self.template, orig_width, orig_height, target_width, target_height);
            img = template.resize(target_width, target_height, FilterType::Lanczos3);
        } else {
            debug!("Using original template size of {}x{}", orig_width, orig_height);
            img = (*template).clone();  // clone the actual image
        }

        let (width, height) = img.dimensions();
        trace!("Final image size: {}x{}", width, height);
        img
    }

    /// Draw the text from ImageMacro on given image.
    /// Returns a new image.
    fn draw_text(&self, img: DynamicImage) -> Result<DynamicImage, CaptionError> {
        // Rendering text requires alpha blending.
        let mut img = img;
        if img.as_rgba8().is_none() {
            trace!("Converting image to RGBA...");
            img = DynamicImage::ImageRgba8(img.to_rgba());
        }

        let font = self.cache.get_font(self.font())
            .ok_or_else(|| CaptionError::Font(self.font().to_owned()))?;

        // TODO: moar constants, better encapsulation, all that jazz
        let size = 64.0;
        if let Some(ref top_text) = self.top_text {
            let alignment = (VAlign::Top, HAlign::Center);
            let top_margin_px = 16.0;
            debug!("Rendering top text: {}", top_text);
            img = text::render_line(
                img, top_text, alignment, vector(0.0, top_margin_px),
                Style::white(&font, size));
        }
        if let Some(ref middle_text) = self.middle_text {
            let alignment = (VAlign::Middle, HAlign::Center);
            debug!("Rendering middle text: {}", middle_text);
            img = text::render_line(
                img, middle_text, alignment, vector(0.0, 0.0),
                Style::white(&font, size));
        }
        if let Some(ref bottom_text) = self.bottom_text {
            let alignment = (VAlign::Bottom, HAlign::Center);
            let bottom_margin_px =  16.0;
            debug!("Rendering bottom text: {}", bottom_text);
            img = text::render_line(
                img, bottom_text, alignment, vector(0.0, -bottom_margin_px),
                Style::white(&font, size));
        }

        Ok(img)
    }

    /// Encode final result as PNG bytes.
    fn encode_image(&self, img: DynamicImage) -> Result<Vec<u8>, CaptionError> {
        debug!("Encoding final image as PNG...");

        let (width, height) = img.dimensions();
        let mut image_bytes = vec![];
        image::png::PNGEncoder::new(&mut image_bytes)
            .encode(&*img.raw_pixels(), width, height, img.color())
            .map_err(CaptionError::Encode)?;

        Ok(image_bytes)
    }
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
