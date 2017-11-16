//! Module for handling command line arguments.

mod image_macro;
mod model;

#[cfg(test)]
mod tests;


use std::env;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use conv::TryFrom;
use clap::{self, AppSettings, Arg, ArgGroup, ArgMatches};
use serde_json;

use super::{NAME, VERSION};
pub use self::model::{ArgsError, Options};
use self::image_macro::parse as parse_image_macro;


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


impl<'a> TryFrom<ArgMatches<'a>> for Options {
    type Err = ArgsError;

    fn try_from(matches: ArgMatches<'a>) -> Result<Self, Self::Err> {
        let verbose_count = matches.occurrences_of(OPT_VERBOSE) as isize;
        let quiet_count = matches.occurrences_of(OPT_QUIET) as isize;
        let verbosity = verbose_count - quiet_count;

        let image_macro = match matches.value_of(ARG_MACRO) {
            Some(im) => parse_image_macro(im.trim())?,
            None => {
                assert!(matches.is_present(OPT_JSON),
                    "Command line incorrectly parsed without either `{}` argument or --{} flag",
                    ARG_MACRO, OPT_JSON);
                serde_json::from_reader(&mut io::stdin())?
            }
        };

        // Output path can be set explicitly to stdout via `-`.
        let output_path = matches.value_of(OPT_OUTPUT)
            .map(|p| p.trim())
            .and_then(|p| if p == "-" { None } else { Some(p) })
            .map(|p| PathBuf::from(p));

        Ok(Options{verbosity, image_macro, output_path})
    }
}


// Parser definition

/// Type of the argument parser object
/// (which is called an "App" in clap's silly nomenclature).
pub type Parser<'p> = clap::App<'p, 'p>;


lazy_static! {
    static ref ABOUT: &'static str = option_env!("CARGO_PKG_DESCRIPTION").unwrap_or("");
}

const ARGGRP_MACRO: &'static str = "image_macro";
const ARG_MACRO: &'static str = "macro";
const OPT_JSON: &'static str = "json";
const OPT_OUTPUT: &'static str = "output";
const OPT_VERBOSE: &'static str = "verbose";
const OPT_QUIET: &'static str = "quiet";


/// Create the parser for application's command line.
pub fn create_parser<'p>() -> Parser<'p> {
    let mut parser = Parser::new(*NAME);
    if let Some(version) = *VERSION {
        parser = parser.version(version);
    }
    parser
        .about(*ABOUT)
        .author(crate_authors!(", "))

        .setting(AppSettings::StrictUtf8)

        .setting(AppSettings::UnifiedHelpMessage)
        .setting(AppSettings::DontCollapseArgsInUsage)
        .setting(AppSettings::DeriveDisplayOrder)

        // Image macro specification.
        .group(ArgGroup::with_name(ARGGRP_MACRO)
            .args(&[ARG_MACRO, OPT_JSON])
            .required(true))  // TODO: make it optional and add interactive option
        .arg(Arg::with_name(ARG_MACRO)
            .value_name("MACRO")
            .help("Image macro to render")
            .long_help(concat!(
                "Specification of the image macro to render.\n\n",
                "The syntax is: TEMPLATE{CAPTION}{CAPTION}..., where CAPTION is just text ",
                "or text preceded by alignment symbols: ^, - (middle), _ (bottom), ",
                "<, | (center), >.")))
        .arg(Arg::with_name(OPT_JSON)
            .conflicts_with(ARG_MACRO)
            .long("json").short("j")
            .help("Whether to expect image macro as JSON on standard input")
            .long_help(concat!(
                "If present, the image macro specification will be read as JSON ",
                "from the program's standard input.")))
            // TODO: some documentation of the JSON format

        // Output flags.
        .arg(Arg::with_name(OPT_OUTPUT)
            .long("output").short("o")
            .required(false)
            .help("File to write the rendered image to")
            .long_help(concat!(
                "What file should the final image be written to.\n\n",
                "By default, or when this flag is set to `-` (single dash), the image is written ",
                "to standard output so it can be e.g. piped to the ImageMagick `display` program.")))

        // Verbosity flags.
        .arg(Arg::with_name(OPT_VERBOSE)
            .long("verbose").short("v")
            .multiple(true)
            .conflicts_with(OPT_QUIET)
            .help("Increase logging verbosity"))
        .arg(Arg::with_name(OPT_QUIET)
            .long("quiet").short("q")
            .multiple(true)
            .conflicts_with(OPT_VERBOSE)
            .help("Decrease logging verbosity"))

        .help_short("H")
        .version_short("V")
}
