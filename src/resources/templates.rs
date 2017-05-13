//! Module handling image macro templates.

use std::env;
use std::fmt;
use std::path::{Path, PathBuf};

use glob;
use image::{self, DynamicImage, GenericImage, ImageFormat};


/// Default image format to use when encoding image macros.
pub const DEFAULT_IMAGE_FORMAT: ImageFormat = ImageFormat::PNG;


/// Represents an image macro template.
#[derive(Clone)]
pub enum Template {
    /// Single still image, loaded from some image format.
    Image(DynamicImage, ImageFormat),
    // TODO: add animated GIFs
}

impl Template {
    pub fn for_image<P: AsRef<Path>>(img: DynamicImage, path: P) -> Self {
        let path = path.as_ref();
        let img_format = path.extension().and_then(|s| s.to_str()).and_then(|ext| {
            let ext = ext.to_lowercase();
            match &ext[..] {
                "jpg" | "jpeg" => Some(ImageFormat::JPEG),
                "png" => Some(ImageFormat::PNG),
                // TODO: gif
                _ => None,
            }
        }).unwrap_or(DEFAULT_IMAGE_FORMAT);

        Template::Image(img, img_format)
    }
}

impl Template {
    pub fn into_images(self) -> Vec<DynamicImage> {
        match self {
            Template::Image(img, ..) => vec![img],
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
        }
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
