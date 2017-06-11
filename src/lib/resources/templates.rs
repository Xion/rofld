//! Module handling image macro templates.

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::iter;
use std::path::Path;

use image::{self, DynamicImage, GenericImage, ImageFormat};

use util::animated_gif::{self, GifAnimation, is_gif, is_gif_animated};
use super::Loader;
use super::filesystem::PathLoader;


/// Default image format to use when encoding image macros.
pub const DEFAULT_IMAGE_FORMAT: ImageFormat = ImageFormat::PNG;

lazy_static! {
    /// Map of template file extensions to supported image formats.
    #[doc(hidden)]
    pub static ref IMAGE_FORMAT_EXTENSIONS: HashMap<&'static str, ImageFormat> = hashmap!{
        "gif" => ImageFormat::GIF,
        "jpeg" => ImageFormat::JPEG,
        "jpg" => ImageFormat::JPEG,
        "png" => ImageFormat::PNG,
    };
}


/// Represents an image macro template.
///
/// Currently, templates can either be regular (still) images,
/// or animations loaded from a GIF file.
#[derive(Clone)]
pub enum Template {
    /// Single still image, loaded from some image format.
    Image(DynamicImage, ImageFormat),
    /// An animation, loaded from a GIF.
    Animation(GifAnimation),
}

impl Template {
    /// Create the template for an image loaded from a file.
    /// Image format is figured out from the file extension.
    pub fn for_image<P: AsRef<Path>>(img: DynamicImage, path: P) -> Self {
        let extension = path.as_ref().extension().and_then(|e| e.to_str())
            .map(|s| s.trim().to_lowercase());
        let img_format = extension
            .and_then(|ext| IMAGE_FORMAT_EXTENSIONS.get(ext.as_str()).map(|f| *f))
            .unwrap_or(DEFAULT_IMAGE_FORMAT);
        Template::Image(img, img_format)
    }

    /// Create the template for an animation loaded from a GIF file.
    #[inline]
    pub fn for_gif_animation(gif_anim: GifAnimation) -> Self {
        Template::Animation(gif_anim)
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


/// Error that may occur during template load.
#[derive(Debug, Error)]
pub enum TemplateError {
    /// Error while loading the template file.
    #[error(msg = "I/O error while loading template")]
    File(io::Error),
    /// Error when opening a template image didn't succeed.
    #[error(msg = "error while opening template image")]
    OpenImage(image::ImageError),
    /// Error when opening a template's animated GIF didn't succeed.
    #[error(msg = "error while opening animated GIF template")]
    DecodeAnimatedGif(animated_gif::DecodeError),
}


/// Loader for templates stored in a directory.
///
/// Template names are translated directly into file names, loaded, and cached.
#[derive(Debug)]
pub struct TemplateLoader {
    inner: PathLoader<'static>,
}

impl TemplateLoader {
    /// Create a new template loader.
    #[inline]
    pub fn new<D: AsRef<Path>>(directory: D) -> Self {
        TemplateLoader{
            inner: PathLoader::for_extensions(directory, IMAGE_FORMAT_EXTENSIONS.keys()),
        }
    }
}

impl Loader for TemplateLoader {
    type Item = Template;
    type Err = TemplateError;

    fn load<'n>(&self, name: &'n str) -> Result<Template, Self::Err> {
        let path = self.inner.load(name)?;

        // Use the `gif` crate to load animated GIFs.
        // Use the regular `image` crate to load any other (still) image.
        if is_gif(&path) && is_gif_animated(&path).unwrap_or(false) {
            trace!("Image {} is an animated GIF", path.display());
            let gif_anim = animated_gif::decode_from_file(&path).map_err(|e| {
                error!("Failed to open animated GIF template {}: {}",
                    path.display(), e); e
            })?;
            Ok(Template::for_gif_animation(gif_anim))
        } else {
            trace!("Opening image {}", path.display());
            let img = image::open(&path)?;
            Ok(Template::for_image(img, &path))
        }
    }
}
