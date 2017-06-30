//! Module defining the command line argument parser.

use std::path::PathBuf;

use conv::TryFrom;
use clap::{self, AppSettings, Arg, ArgMatches};

use super::{NAME, VERSION};
use super::image_macro::parse as parse_image_macro;
use super::model::{ArgsError, Options};


impl<'a> TryFrom<ArgMatches<'a>> for Options {
    type Err = ArgsError;

    fn try_from(matches: ArgMatches<'a>) -> Result<Self, Self::Err> {
        let verbose_count = matches.occurrences_of(OPT_VERBOSE) as isize;
        let quiet_count = matches.occurrences_of(OPT_QUIET) as isize;
        let verbosity = verbose_count - quiet_count;

        let image_macro = {
            let im = matches.value_of(ARG_MACRO).unwrap().trim();
            parse_image_macro(im)?
        };

        // Output path can be set explicit to stdout via `-`.
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

const ARG_MACRO: &'static str = "macro";
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
        .arg(Arg::with_name(ARG_MACRO)
            .value_name("MACRO")
            .required(true)  // TODO: make it optional and add interactive option
            .help("Image macro to render")
            .long_help(concat!(
                "Specification of the image macro to render.\n\n",
                "The syntax is: TEMPLATE{CAPTION}{CAPTION}..., where CAPTION is just text ",
                "or text preceded by alignment symbols: ^ - (middle), _ (bottom), ",
                "<, | (center), >. (Vertical alignment must preceed horizontal alignment).")))
        // TODO: --json option to allow the image macro spec to be given as JSON
        // (by default on stdin rather than as argument)

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
