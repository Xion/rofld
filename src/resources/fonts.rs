//! Module for loading fonts used in image macros.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use glob;
use rusttype::{Font, FontCollection};


lazy_static! {
    static ref FONT_DIR: PathBuf =
        env::current_dir().unwrap().join("data").join("fonts");
}

const FILE_EXTENSION: &'static str = "ttf";


/// Load the font with given name.
pub fn load<'f>(name: &str) -> Option<Font<'f>> {
    debug!("Loading font `{}`...", name);

    let path = FONT_DIR.join(format!("{}.{}", name, FILE_EXTENSION));
    let file = try_opt!(fs::File::open(&path).map_err(|e| {
        error!("Failed to open font file `{}`: {}", path.display(), e); e
    }).ok());

    // Read the font file into a byte buffer.
    let mut bytes = match file.metadata() {
        Ok(stat) => Vec::with_capacity(stat.len() as usize),
        Err(e) => {
            warn!("Failed to stat font file `{}` to obtain its size: {}",
                path.display(), e);
            Vec::new()
        },
    };
    let mut reader = BufReader::new(file);
    try_opt!(reader.read_to_end(&mut bytes).map_err(|e| {
        error!("Failed to read content of font file `{}`: {}", path.display(), e); e
    }).ok());

    let fonts: Vec<_> = FontCollection::from_bytes(bytes).into_fonts().collect();
    match fonts.len() {
        0 => { error!("Alleged font file `{}` contains no fonts", path.display()); None },
        1 => {
            debug!("Font `{}` loaded successfully", name);
            fonts.into_iter().next()
        },
        _ => {
            error!("Font file `{}` contains {} fonts, expected one",
                path.display(), fonts.len());
            None
        },
    }
}

/// List all available font names.
pub fn list() -> Vec<String> {
    debug!("Listing all available fonts...");

    let pattern = format!("{}",
        FONT_DIR.join(&format!("*.{}", FILE_EXTENSION)).display());
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
