//! Module handling image macro templates.

use std::collections::{HashSet, HashMap};
use std::env;
use std::fmt;
use std::iter;
use std::path::{Path, PathBuf};

use conv::TryFrom;
use glob;
use image::{self, DynamicImage, GenericImage, ImageFormat};

use util::animated_gif::{self, GifAnimation, is_gif, is_gif_animated};


/// Default image format to use when encoding image macros.
pub const DEFAULT_IMAGE_FORMAT: ImageFormat = ImageFormat::PNG;

lazy_static! {
    /// Map of template file extensions to supported image formats.
    static ref IMAGE_FORMAT_EXTENSIONS: HashMap<&'static str, ImageFormat> = hashmap!{
        "gif" => ImageFormat::GIF,
        "jpeg" => ImageFormat::JPEG,
        "jpg" => ImageFormat::JPEG,
        "png" => ImageFormat::PNG,
    };
}


/// Represents an image macro template.
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
        let img_format = extension(path)
            .and_then(|ext| IMAGE_FORMAT_EXTENSIONS.get(ext.as_str()).map(|f| *f))
            .unwrap_or(DEFAULT_IMAGE_FORMAT);
        Template::Image(img, img_format)
    }

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


// Loading templates

impl<P: AsRef<Path>> TryFrom<P> for Template {
    type Err = TemplateError;

    fn try_from(path: P) -> Result<Self, Self::Err> {
        let path = path.as_ref();

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

macro_attr! {
    #[derive(Debug,
             Error!("template loading error"), ErrorDisplay!, ErrorFrom!)]
    pub enum TemplateError {
        OpenImage(image::ImageError),
        DecodeAnimatedGif(animated_gif::DecodeError),
    }
}


lazy_static! {
    static ref TEMPLATE_DIR: PathBuf =
        env::current_dir().unwrap().join("data").join("templates");
}

/// Load an image macro template.
pub fn load(template: &str) -> Option<Template> {
    debug!("Loading image macro template `{}`", template);

    // TODO: what about ambiguous file stems? (same template name, different extension)
    let template_path = try_opt!(glob_templates(template).next());
    trace!("Path to image for template {} is {}", template, template_path.display());

    match Template::try_from(&template_path) {
        Ok(t) => {
            debug!("Template `{}` loaded successfully", template);
            Some(t)
        }
        Err(e) => {
            error!("Failed to load template `{}` from {}: {}",
                template, template_path.display(), e);
            None
        }
    }
}


// Other

/// List all available template names.
pub fn list() -> Vec<String> {
    debug!("Listing all available templates...");
    let templates = glob_templates("*")
        .fold(HashSet::new(), |mut ts, t| {
            let name = t.file_stem().unwrap().to_str().unwrap().to_owned();
            ts.insert(name); ts
        });

    debug!("{} template(s) found", templates.len());
    let mut result: Vec<_> = templates.into_iter().collect();
    result.sort();
    result
}


// Utility functions

/// Yield paths to template files that have the given file stem.
fn glob_templates(stem: &str) -> Box<Iterator<Item=PathBuf>> {
    let file_part = format!("{}.*", stem);
    let pattern = format!("{}", TEMPLATE_DIR.join(file_part).display());
    trace!("Globbing with {}", pattern);

    let glob_iter = match glob::glob(&pattern) {
        Ok(it) => it,
        Err(e) => {
            error!("Failed to glob over template files: {}", e);
            return Box::new(iter::empty());
        },
    };

    // We manually filter out unsupported file extensions because the `glob` crate
    // doesn't support patterns like foo.{gif|png} (i.e. with braces).
    Box::new(glob_iter
        .filter_map(Result::ok)  // TODO: report errors about this
        .filter(|f| {
            let ext = extension(f);
            IMAGE_FORMAT_EXTENSIONS.keys()
                .any(|&e| Some(e) == ext.as_ref().map(|e| e.as_str()))
        }))
}

/// Get the (useful part of) file extension from the path.
fn extension<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref().extension().and_then(|e| e.to_str())
        .map(|s| s.trim().to_lowercase())
}
