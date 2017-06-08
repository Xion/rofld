//!
//! rofl  -- Lulz on demand
//!

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
pub use util::cache::*;
