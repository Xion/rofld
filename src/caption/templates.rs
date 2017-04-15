//! Module handling image macro templates.

use std::env;
use std::path::PathBuf;

use glob;
use image;


lazy_static! {
    static ref TEMPLATE_DIR: PathBuf =
        env::current_dir().unwrap().join("data").join("templates");
}


/// Load template image.
pub fn load(template: &str) -> Option<image::DynamicImage> {
    debug!("Loading image macro template `{}`", template);

    // TODO: cache templates
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
            Some(i)
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
