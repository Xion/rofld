//! Module with the handlers for listing available resources.

use std::collections::HashSet;
use std::iter;
use std::path::{Path, PathBuf};

use glob;
use rofl::{FONT_FILE_EXTENSION, IMAGE_FORMAT_EXTENSIONS};

use super::{TEMPLATE_DIR, FONT_DIR};


/// List all available font names.
pub fn list_fonts() -> Vec<String> {
    debug!("Listing all available fonts...");

    let pattern = format!("{}",
        FONT_DIR.join(&format!("*.{}", FONT_FILE_EXTENSION)).display());
    trace!("Globbing with {}", pattern);
    let fonts = glob::glob(&pattern).unwrap()
        .filter_map(Result::ok)  // TODO: report errors about this
        .fold(HashSet::new(), |mut ts, t| {
            let name = t.file_stem().unwrap().to_str().unwrap().to_owned();
            ts.insert(name); ts
        });

    debug!("{} font(s) found", fonts.len());
    let mut result: Vec<_> = fonts.into_iter().collect();
    result.sort();
    result
}


/// List all available template names.
pub fn list_templates() -> Vec<String> {
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
