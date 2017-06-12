//! *Lulz on demand!*
//!
//! This here `rofl` crate, aptly named, is capable of the extraordinary feat
//! of putting text on images. And not just still images: it does cover animated GIFs as well!
//!
//! In other words, the crate can be used to create _memes_,
//! which purists generally refer to as _image macros_.
//!
//! # Much example
//!
//! ```rust
//! extern crate rofl;
//!
//! # use std::error::Error;
//! # use std::io::Write;
//! # use std:: fs;
//! #
//! # fn zoidberg() -> Result<(), Box<Error>> {
//! let engine = rofl::Engine::new("data/templates", "data/fonts");
//! let image_macro = rofl::ImageMacro {
//!     template: "zoidberg".into(),
//!     captions: vec![
//!         rofl::Caption::text_at(rofl::VAlign::Top, "Need an example?"),
//!         rofl::Caption::text_at(rofl::VAlign::Bottom, "Why not Zoidberg?"),
//!     ],
//!     ..rofl::ImageMacro::default()
//! };
//! let output = engine.caption(image_macro)?;
//!
//! let mut file = fs::OpenOptions::new().write(true).open("zoidberg.png")?;
//! file.write_all(&*output)?;
//! #   Ok(())
//! # }
//! ```
//!
//! # Very concepts
//!
//! To create memes, you need two types of media resources (in addition to impeccable wit):
//!
//! * _templates_ -- named images & animated GIFs that we can put text on
//! * _fonts_ to render the text with (like `"Impact"` or `"Comic Sans"`)
//!
//! Those resources have to be provided to the captioning [`Engine`](struct.Engine.html).
//!
//! In the simple above, they are just files contained within some directories.
//! If you're doing something more complicated --
//! like a website where users can upload their own images --
//! you can implement your own [`Loader`s](trait.Loader.html) for templates or even fonts.
//!
//! A meme is defined by [the `ImageMacro` structure](struct.ImageMacro.html).
//! These can be deserialized from JSON or query strings if desired.
//!
//! # Wow
//!
//! Go forth and meme!


#![deny(missing_docs)]


             extern crate color_quant;
             extern crate conv;
             extern crate css_color_parser;
#[macro_use] extern crate derive_error;
#[macro_use] extern crate enum_derive;
             extern crate gif;
             extern crate gif_dispose;
             extern crate glob;
             extern crate image;
             extern crate itertools;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
             extern crate lru_cache;
#[macro_use] extern crate macro_attr;
#[macro_use] extern crate maplit;
             extern crate mime;
#[macro_use] extern crate newtype_derive;
             extern crate num;
             extern crate rand;
             extern crate regex;
             extern crate rusttype;
             extern crate serde;
#[macro_use] extern crate serde_derive;
             extern crate time;
#[macro_use] extern crate try_opt;
             extern crate unicode_normalization;
             extern crate unreachable;


#[cfg(test)] #[macro_use] extern crate serde_json;
#[cfg(test)]              extern crate serde_qs;
#[cfg(test)]              extern crate serde_test;
#[cfg(test)] #[macro_use] extern crate spectral;


mod caption;
mod model;
mod resources;
mod util;


pub use caption::*;
pub use model::*;
pub use resources::*;
pub use util::{animated_gif, cache};
