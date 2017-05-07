//! Module handling image macro templates.

use std::env;
use std::fmt;
use std::path::PathBuf;

use glob;
use image::{self, GenericImage};


custom_derive! {
    /// Represents an image macro template.
    #[derive(Clone, EnumFromInner)]
    pub enum Template {
        Image(image::DynamicImage),
        // TODO: add animated GIFs
    }
}
impl Template {
    pub fn into_images(self) -> Vec<image::DynamicImage> {
        match self {
            Template::Image(img) => vec![img],
        }
    }
}
impl fmt::Debug for Template {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Template::Image(ref img) => {
                let (width, height) = img.dimensions();
                write!(fmt, "Template::Image({}x{})", width, height)
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
        Ok(i) => {
            debug!("Template `{}` opened successfully", template);
            Some(i.into())
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
