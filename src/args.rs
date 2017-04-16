//! Module for handling command line arguments.

use std::env;
use std::ffi::OsString;

use clap::{self, AppSettings, Arg, ArgMatches, ArgSettings};
use conv::TryFrom;

use super::{NAME, VERSION};


/// Parse command line arguments and return Options' object.
#[inline]
pub fn parse() -> clap::Result<Options> {
    parse_from_argv(env::args_os())
}

/// Parse application options from given array of arguments
/// (*all* arguments, including binary name).
#[inline]
pub fn parse_from_argv<I, T>(argv: I) -> clap::Result<Options>
    where I: IntoIterator<Item=T>, T: Clone + Into<OsString>
{
    let parser = create_parser();
    let matches = try!(parser.get_matches_from_safe(argv));
    Options::try_from(matches)
}


/// Structure to hold options received from the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Verbosity of the logging output.
    ///
    /// Corresponds to the number of times the -v flag has been passed.
    /// If -q has been used instead, this will be negative.
    pub verbosity: isize,

    // TODO: more options
}

#[allow(dead_code)]
impl Options {
    #[inline]
    pub fn verbose(&self) -> bool { self.verbosity > 0 }
    #[inline]
    pub fn quiet(&self) -> bool { self.verbosity < 0 }
}

impl<'a> TryFrom<ArgMatches<'a>> for Options {
    type Err = clap::Error;

    fn try_from(matches: ArgMatches<'a>) -> Result<Self, Self::Err> {
        let verbose_count = matches.occurrences_of(OPT_VERBOSE) as isize;
        let quiet_count = matches.occurrences_of(OPT_QUIET) as isize;
        let verbosity = verbose_count - quiet_count;

        Ok(Options{
            verbosity: verbosity,
        })
    }
}


// Parser configuration

/// Type of the argument parser object
/// (which is called an "App" in clap's silly nomenclature).
type Parser<'p> = clap::App<'p, 'p>;


lazy_static! {
    static ref ABOUT: &'static str = option_env!("CARGO_PKG_DESCRIPTION").unwrap_or("");
}

const OPT_VERBOSE: &'static str = "verbose";
const OPT_QUIET: &'static str = "quiet";


/// Create the parser for application's command line.
fn create_parser<'p>() -> Parser<'p> {
    let mut parser = Parser::new(*NAME);
    if let Some(version) = *VERSION {
        parser = parser.version(version);
    }
    parser
        .about(*ABOUT)

        .setting(AppSettings::UnifiedHelpMessage)
        .setting(AppSettings::DeriveDisplayOrder)
        .setting(AppSettings::ColorNever)

        // Verbosity flags.
        .arg(Arg::with_name(OPT_VERBOSE)
            .long("verbose").short("v")
            .set(ArgSettings::Multiple)
            .conflicts_with(OPT_QUIET)
            .help("Increase logging verbosity"))
        .arg(Arg::with_name(OPT_QUIET)
            .long("quiet").short("q")
            .set(ArgSettings::Multiple)
            .conflicts_with(OPT_VERBOSE)
            .help("Decrease logging verbosity"))

        .help_short("H")
        .version_short("V")
}
