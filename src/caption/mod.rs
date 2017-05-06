//! Module implementing the actual image captioning.

mod text;


use std::error::Error;
use std::fmt;
use std::io;
use std::ops::Deref;
use std::sync::{Arc, Mutex, TryLockError};
use std::time::Duration;

use atomic::{Atomic, Ordering};
use futures::{BoxFuture, Future, future};
use futures_cpupool::{self, CpuPool};
use hyper::StatusCode;
use image::{self, DynamicImage, FilterType, GenericImage};
use rusttype::{point, Rect, vector};
use tokio_timer::{Timer, TimeoutError, TimerError};

use model::{Caption, ImageMacro};
use resources::Cache;
use self::text::Style;


/// Renders image macros into captioned images.
pub struct Captioner {
    pool: Mutex<CpuPool>,
    cache: Arc<Cache>,
    timer: Timer,
    // Configuration params.
    task_timeout: Atomic<Duration>,
}

impl Captioner {
    #[inline]
    fn new() -> Self {
        let pool = Mutex::new(Self::pool_builder().create());
        let cache = Arc::new(Cache::new());
        let timer = Timer::default();

        let task_timeout = Atomic::new(Duration::from_secs(0));

        Captioner{pool, cache, timer, task_timeout}
    }

    #[inline]
    #[doc(hidden)]
    fn pool_builder() -> futures_cpupool::Builder {
        let mut builder = futures_cpupool::Builder::new();
        builder.name_prefix("caption-");
        builder.after_start(|| trace!("Worker thread created in Captioner::pool"));
        builder.before_stop(|| trace!("Stopping worker a thread from Captioner::pool"));
        builder
    }
}

impl Captioner {
    #[inline]
    pub fn cache(&self) -> &Cache {
        &*self.cache
    }
}

// Configuration tweaks.
impl Captioner {
    #[inline]
    pub fn set_thread_count(&self, count: usize) -> &Self {
        trace!("Setting thread count for image captioning to {}", count);

        let mut builder = Self::pool_builder();
        if count > 0 {
            builder.pool_size(count);
        }

        let pool = builder.create();
        *self.pool.lock().unwrap() = pool;
        self
    }

    #[inline]
    pub fn set_task_timeout(&self, timeout: Duration) -> &Self {
        trace!("Setting caption request timeout to {} secs", timeout.as_secs());
        self.task_timeout.store(timeout, Ordering::Relaxed);
        self
    }
}

// Rendering code.
impl Captioner {
    /// Render an image macro as PNG.
    /// The rendering is done in a separate thread.
    pub fn render(&self, im: ImageMacro) -> BoxFuture<Vec<u8>, CaptionError> {
        let pool = match self.pool.try_lock() {
            Ok(p) => p,
            Err(TryLockError::WouldBlock) => {
                // This should be only possible when set_thread_count() happens
                // to have been called at the exact same moment.
                warn!("Could not immediately lock CpuPool to render {:?}", im);
                // TODO: retry a few times, probably with exponential backoff
                return future::err(CaptionError::Unavailable).boxed();
            },
            Err(e) => {
                // TODO: is this a fatal error?
                error!("Error while locking CpuPool for rendering {:?}: {}", im, e);
                return future::err(CaptionError::Unavailable).boxed();
            },
        };

        // Spawn a new task in the thread pool for the rendering process.
        let task_future = pool.spawn_fn({
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
        });

        // Impose a timeout on the task.
        let max_duration = self.task_timeout.load(Ordering::Relaxed);
        if max_duration.as_secs() > 0 {
            self.timer.timeout(task_future, max_duration).boxed()
        } else {
            task_future.boxed()
        }
    }
}

lazy_static! {
    /// The singleton instance of Captioner.
    pub static ref CAPTIONER: Arc<Captioner> = Arc::new(Captioner::new());
}

/// Represents a single captioning task and contains all the relevant logic.
///
/// This is a separate struct so that the rendering state (e.g. the cache)
/// can be easily carried between its methods.
///
/// All the code here is executed in a background thread,
/// and so it can be synchronous.
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
            img = self.draw_texts(img)?;
        }
        self.encode_image(img)
    }

    /// Resize template image to fit the desired dimensions.
    fn resize_template(&self, template: Arc<DynamicImage>) -> DynamicImage {
        // Note that resizing preserves original aspect, so the final image
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
    fn draw_texts(&self, img: DynamicImage) -> Result<DynamicImage, CaptionError> {
        // Rendering text requires alpha blending.
        let mut img = img;
        if img.as_rgba8().is_none() {
            trace!("Converting image to RGBA...");
            img = DynamicImage::ImageRgba8(img.to_rgba());
        }

        for cap in &self.captions {
            img = self.draw_single_caption(img, cap)?;
        }

        Ok(img)
    }

    /// Draws a single caption text.
    /// Returns a new image.
    fn draw_single_caption(&self, img: DynamicImage,
                           caption: &Caption) -> Result<DynamicImage, CaptionError> {
        let mut img = img;

        if caption.text.is_empty() {
            debug!("Empty caption text, skipping.");
            return Ok(img);
        }
        debug!("Rendering {v}-{h} text: {text}", text=caption.text,
            v=format!("{:?}", caption.valign).to_lowercase(),
            h=format!("{:?}", caption.halign).to_lowercase());

        trace!("Loading font `{}` from cache...", caption.font);
        let font = self.cache.get_font(&caption.font)
            .ok_or_else(|| CaptionError::Font(caption.font.clone()))?;

        let (_, _, width, height) = img.bounds();
        let width = width as f32;
        let height = height as f32;

        // Make sure the vertical margin isn't too large by limiting it
        // to a small percentage of image height.
        let max_vmargin: f32 = 16.0;
        let vmargin = max_vmargin.signum() * max_vmargin.abs().min(height * 0.02);
        trace!("Vertical text margin computed as {}", vmargin);

        // Similarly for the horizontal margin.
        let max_hmargin: f32 = 16.0;
        let hmargin = max_hmargin.signum() * max_hmargin.abs().min(height * 0.02);
        trace!("Horizontal text margin computed as {}", hmargin);

        let margin_vector = vector(hmargin, vmargin);
        let rect: Rect<f32> = Rect{
            min: point(0.0, 0.0) + margin_vector,
            max: point(width, height) - margin_vector,
        };

        let alignment = (caption.halign, caption.valign);

        // TODO: either make this an ImageMacro parameter,
        // or allow to choose between Wrap and Shrink methods of text fitting
        let text_size = 64.0;

        // Draw four copies of the text, shifted in four diagonal directions,
        // to create the basis for an outline.
        if let Some(outline_color) = caption.outline {
            let outline_width = 2.0;
            for &v in [vector(-outline_width, -outline_width),
                       vector(outline_width, -outline_width),
                       vector(outline_width, outline_width),
                       vector(-outline_width, outline_width)].iter() {
                let style = Style::new(&font, text_size, outline_color);
                let rect = Rect{min: rect.min + v, max: rect.max + v};
                img = text::render_text(img, &caption.text, alignment, rect, style);
            }
        }

        // Now render the white text in the original position.
        let style = Style::new(&font, text_size, caption.color);
        img = text::render_text(img, &caption.text, alignment, rect, style);

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
    // Errors related to rendering logic.
    Template(String),
    Font(String),
    Encode(io::Error),

    // Other.
    Timeout,
    Unavailable,
}
unsafe impl Send for CaptionError {}

impl CaptionError {
    #[inline]
    pub fn status_code(&self) -> StatusCode {
        match *self {
            CaptionError::Template(..) => StatusCode::NotFound,
            CaptionError::Font(..) => StatusCode::NotFound,
            CaptionError::Encode(..) => StatusCode::InternalServerError,
            CaptionError::Timeout => StatusCode::InternalServerError,
            CaptionError::Unavailable => StatusCode::ServiceUnavailable,
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
            CaptionError::Timeout => write!(fmt, "caption task timed out"),
            CaptionError::Unavailable => write!(fmt, "captioning currently unavailable"),
        }
    }
}

// Necessary for imposing a timeout on the CaptionTask.
impl<F> From<TimeoutError<F>> for CaptionError {
    fn from(e: TimeoutError<F>) -> Self {
        match e {
            TimeoutError::Timer(_, TimerError::NoCapacity) => CaptionError::Unavailable,
            _ => CaptionError::Timeout,
        }
    }
}
