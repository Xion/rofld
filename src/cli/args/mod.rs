//! Module for handling command line arguments.

mod image_macro;
mod model;
mod parser;


use std::env;
use std::ffi::OsString;

use conv::TryFrom;

use super::{NAME, VERSION};
pub use self::model::{ArgsError, Options};
use self::parser::create_parser;


/// Parse command line arguments and return `Options` object.
#[inline]
pub fn parse() -> Result<Options, ArgsError> {
    parse_from_argv(env::args_os())
}

/// Parse application options from given array of arguments
/// (*all* arguments, including binary name).
#[inline]
pub fn parse_from_argv<I, T>(argv: I) -> Result<Options, ArgsError>
    where I: IntoIterator<Item=T>, T: Clone + Into<OsString>
{
    let parser = create_parser();
    let matches = try!(parser.get_matches_from_safe(argv));
    Options::try_from(matches)
}
