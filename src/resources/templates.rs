//! Module handling image macro templates.

use std::env;
use std::fmt;
use std::fs::File;
use std::iter;
use std::path::{Path, PathBuf};

use gif::{self, SetParameter};
use glob;
use image::{self, DynamicImage, GenericImage, ImageFormat, RgbaImage};


/// Default image format to use when encoding image macros.
pub const DEFAULT_IMAGE_FORMAT: ImageFormat = ImageFormat::PNG;


/// Represents an image macro template.
#[derive(Clone)]
pub enum Template {
    /// Single still image, loaded from some image format.
    Image(DynamicImage, ImageFormat),
    /// An animation, loaded from a GIF.
    Animation(GifAnimation),
}

impl Template {
    pub fn for_image<P: AsRef<Path>>(img: DynamicImage, path: P) -> Self {
        let path = path.as_ref();
        let img_format = path.extension().and_then(|s| s.to_str()).and_then(|ext| {
            let ext = ext.to_lowercase();
            match &ext[..] {
                "jpg" | "jpeg" => Some(ImageFormat::JPEG),
                "png" => Some(ImageFormat::PNG),
                "gif" => Some(ImageFormat::GIF),
                _ => None,
            }
        }).unwrap_or(DEFAULT_IMAGE_FORMAT);

        Template::Image(img, img_format)
    }
}

impl Template {
    /// Whether this is an animated template.
    #[inline]
    pub fn is_animated(&self) -> bool {
        match *self { Template::Animation(..) => true, _ => false, }
    }

    /// Number of images that comprise the template
    #[inline]
    pub fn image_count(&self) -> usize {
        match *self {
            Template::Image(..) => 1,
            Template::Animation(ref gif_anim) => gif_anim.frames_count(),
        }
    }

    /// Iterate over all DynamicImages in this template.
    pub fn iter_images<'t>(&'t self) -> Box<Iterator<Item=&'t DynamicImage> + 't> {
        match *self {
            Template::Image(ref img, ..) => Box::new(iter::once(img)),
            Template::Animation(ref gif_anim) => Box::new(
                gif_anim.iter_frames().map(|f| &f.image)),
        }
    }

    /// The preferred format for image macros generated using this template.
    /// This is usually the same that the template was loaded from.
    pub fn preferred_format(&self) -> ImageFormat {
        match *self {
            Template::Image(_, fmt) => match fmt {
                // These are the formats that image crate encodes natively.
                ImageFormat::PNG | ImageFormat::JPEG => return fmt,
                _ => {}
            },
            Template::Animation(..) => return ImageFormat::GIF,
        }
        DEFAULT_IMAGE_FORMAT
    }
}

impl fmt::Debug for Template {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Template::Image(ref img, f) => {
                let (width, height) = img.dimensions();
                write!(fmt, "Template::Image({}x{}, {:?})", width, height, f)
            }
            Template::Animation(ref gif_anim) => {
                write!(fmt, "Template::Animation({} frame(s))", gif_anim.frames_count())
            }
        }
    }
}


/// Animation loaded from a GIF file.
#[derive(Clone)]
pub struct GifAnimation {
    /// Width of the animation canvas (logical screen).
    pub width: u16,
    /// Height of the animation canvas (logical screen).
    pub height: u16,
    /// Global palette (Color Table).
    pub palette: Vec<u8>,
    /// Index of the background color in the global palette.
    pub bg_color: Option<usize>,
    /// Animation frames.
    frames: Vec<GifFrame>,
}
impl GifAnimation {
    #[inline]
    pub fn frames_count(&self) -> usize {
        self.frames.len()
    }

    #[inline]
    pub fn iter_frames<'a>(&'a self) -> Box<Iterator<Item=&'a GifFrame> + 'a> {
        Box::new(self.frames.iter())
    }
}

/// A single frame of an animated GIF template.
#[derive(Clone)]
pub struct GifFrame {
    /// The image of the frame.
    pub image: DynamicImage,
    /// gif::Frame structure containing just the metadata of the frame.
    /// The actual buffer is emptied and converted into the image.
    pub metadata: gif::Frame<'static>,
}
impl<'f> From<&'f gif::Frame<'f>> for GifFrame {
    fn from(gif_frame: &'f gif::Frame<'f>) -> Self {
        let image = DynamicImage::ImageRgba8(
            RgbaImage::from_raw(
                gif_frame.width as u32, gif_frame.height as u32,
                gif_frame.buffer.to_vec()).unwrap());
        let metadata = gif::Frame{
            buffer: vec![].into(),
            // Copy the rest of the metadata.
            delay: gif_frame.delay,
            dispose: gif_frame.dispose,
            transparent: gif_frame.transparent,
            needs_user_input: gif_frame.needs_user_input,
            top: gif_frame.top,
            left: gif_frame.left,
            width: gif_frame.width,
            height: gif_frame.height,
            interlaced: gif_frame.interlaced,
            palette: gif_frame.palette.clone(),
        };

        GifFrame{image, metadata}
    }
}


lazy_static! {
    static ref TEMPLATE_DIR: PathBuf =
        env::current_dir().unwrap().join("data").join("templates");
}


/// Load an image macro template.
pub fn load(template: &str) -> Option<Template> {
    debug!("Loading image macro template `{}`", template);

    let template_glob = &format!(
        "{}", TEMPLATE_DIR.join(template.to_owned() + ".*").display());
    let mut template_iter = match glob::glob(template_glob) {
        Ok(it) => it,
        Err(e) => {
            error!("Failed to glob over template files: {}", e);
            return None;
        },
    };
    let template_path = try_opt!(template_iter.next().and_then(|p| p.ok()));
    trace!("Path to image for template {} is {}", template, template_path.display());

    // Use the `gif` crate to load animated GIFs.
    // Use the regular `image` crate to load any other (still) image.
    if is_gif(&template_path) && is_gif_animated(&template_path).unwrap_or(false) {
        trace!("Image {} is an animated GIF", template_path.display());
        let gif_anim = try_opt!(load_animated_gif(&template_path).map_err(|e| {
            error!("Failed to open animated GIF template {}: {}",
                template_path.display(), e); e
        }).ok());
        Some(Template::Animation(gif_anim))
    } else {
        trace!("Opening image {}", template_path.display());
        match image::open(&template_path) {
            Ok(img) => {
                debug!("Template `{}` opened successfully", template);
                Some(Template::for_image(img, &template_path))
            },
            Err(e) => {
                error!("Failed to open template image file {}: {}",
                    template_path.display(), e);
                None
            },
        }
    }
}

/// List all available template names.
pub fn list() -> Vec<String> {
    debug!("Listing all available templates...");

    let pattern = format!("{}", TEMPLATE_DIR.join("*.*").display());
    trace!("Globbing with {}", pattern);
    let templates = glob::glob(&pattern).unwrap()
        .filter_map(Result::ok)  // TODO: report errors about this
        .fold(vec![], |mut ts, t| {
            let name = t.file_stem().unwrap().to_str().unwrap().to_owned();
            ts.push(name); ts
        });

    debug!("{} template(s) found", templates.len());
    templates
}


// Loading animated GIFs

// TODO: server command line param
const MEMORY_LIMIT: gif::MemoryLimit = gif::MemoryLimit(32 * 1024 * 1024);

/// Check if the path points to a GIF file.
fn is_gif<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    trace!("Checking if {} is a GIF", path.display());
    path.extension().and_then(|s| s.to_str())
        .map(|ext| ext.to_lowercase() == "gif").unwrap_or(false)
}

/// Check if given GIF image is animated.
/// Returns None if it cannot be determined (e.g. file doesn't exist).
fn is_gif_animated<P: AsRef<Path>>(path: P) -> Option<bool> {
    let path = path.as_ref();
    trace!("Checking if {} is an animated GIF", path.display());

    let mut file = try_opt!(File::open(path).map_err(|e| {
        warn!("Failed to open file {} to check if it's animated GIF: {}",
            path.display(), e); e
    }).ok());

    // The `image` crate technically has an ImageDecoder::is_nimated method,
    // but it doesn't seem to actually work.
    // So instead we just check if the GIF has at least two frames.

    let mut decoder = gif::Decoder::new(&mut file);
    decoder.set(MEMORY_LIMIT);;
    let mut reader = try_opt!(decoder.read_info().ok());

    let mut frame_count = 0;
    while let Some(frame) = try_opt!(reader.next_frame_info().ok()) {
        frame_count += 1;
        if frame_count > 1 && frame.delay > 0 {
            return Some(true);
        }
    }
    Some(false)
}

/// Decode animated GIF from given file.
fn load_animated_gif<P: AsRef<Path>>(path: P) -> Result<GifAnimation, gif::DecodingError> {
    let path = path.as_ref();
    trace!("Loading animated GIF from {}", path.display());
    let mut file = File::open(path).map_err(gif::DecodingError::Io)?;

    let mut decoder = gif::Decoder::new(&mut file);
    decoder.set(gif::ColorOutput::RGBA);
    decoder.set(MEMORY_LIMIT);
    let mut reader = decoder.read_info()?;

    let width = reader.width();
    let height = reader.height();
    let palette = reader.global_palette()
        .map(|p| p.to_vec()).unwrap_or_else(Vec::new);
    let bg_color = reader.bg_color();

    let mut frames = vec![];
    while let Some(gif_frame) = reader.read_next_frame()? {
        frames.push(GifFrame::from(gif_frame));
    }

    Ok(GifAnimation{width, height, palette, bg_color, frames})
}
