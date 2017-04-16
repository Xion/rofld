//! Module for handling command line arguments.

use std::borrow::Cow;
use std::env;
use std::ffi::OsString;
use std::net::{AddrParseError, SocketAddr};

use clap::{self, AppSettings, Arg, ArgMatches, ArgSettings};
use conv::TryFrom;

use super::{NAME, VERSION};


/// Parse command line arguments and return Options' object.
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


/// Structure to hold options received from the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Verbosity of the logging output.
    ///
    /// Corresponds to the number of times the -v flag has been passed.
    /// If -q has been used instead, this will be negative.
    pub verbosity: isize,
    /// Address where the server should listen on.
    pub address: SocketAddr,
}

#[allow(dead_code)]
impl Options {
    #[inline]
    pub fn verbose(&self) -> bool { self.verbosity > 0 }
    #[inline]
    pub fn quiet(&self) -> bool { self.verbosity < 0 }
}

impl<'a> TryFrom<ArgMatches<'a>> for Options {
    type Err = ArgsError;

    fn try_from(matches: ArgMatches<'a>) -> Result<Self, Self::Err> {
        let verbose_count = matches.occurrences_of(OPT_VERBOSE) as isize;
        let quiet_count = matches.occurrences_of(OPT_QUIET) as isize;
        let verbosity = verbose_count - quiet_count;

        let address: SocketAddr = {
            let mut addr: Cow<_> = matches.value_of(ARG_ADDR).unwrap().into();

            // Address can be just a port (e.g. ":4242"),
            // in which case we prepend it with the default host.
            let is_just_port = addr.starts_with(":")
                && !addr.starts_with("::");  // eliminates IPv6 addresses like "::1"
            if is_just_port {
                addr = format!("{}{}", DEFAULT_HOST, addr).into();
            }

            // XXX: this doesn't play well with IPv6; we need to have separate
            // host & port args
            if !addr.contains(":") || addr.contains("::") {
                addr = format!("{}:{}", addr, DEFAULT_PORT).into();
            }

            try!(addr.parse())
        };

        Ok(Options{
            verbosity: verbosity,
            address: address,
        })
    }
}

custom_derive! {
    /// Error that can occur while parsing of command line arguments.
    #[derive(Debug,
             Error("command line arguments error"), ErrorDisplay, ErrorFrom)]
    pub enum ArgsError {
        /// General when parsing the arguments.
        Parse(clap::Error),
        /// Error while parsing the server address.
        Address(AddrParseError),
    }
}


// Parser configuration

/// Type of the argument parser object
/// (which is called an "App" in clap's silly nomenclature).
type Parser<'p> = clap::App<'p, 'p>;


lazy_static! {
    static ref ABOUT: &'static str = option_env!("CARGO_PKG_DESCRIPTION").unwrap_or("");
}

const ARG_ADDR: &'static str = "address";
const OPT_VERBOSE: &'static str = "verbose";
const OPT_QUIET: &'static str = "quiet";

const DEFAULT_HOST: &'static str = "0.0.0.0";
const DEFAULT_PORT: u16 = 1337;
lazy_static! {
    static ref DEFAULT_ADDR: String = format!("{}:{}", DEFAULT_HOST, DEFAULT_PORT);
}


/// Create the parser for application's command line.
fn create_parser<'p>() -> Parser<'p> {
    let mut parser = Parser::new(*NAME);
    if let Some(version) = *VERSION {
        parser = parser.version(version);
    }
    parser
        .about(*ABOUT)
        .author(crate_authors!(", "))

        .setting(AppSettings::UnifiedHelpMessage)
        .setting(AppSettings::DeriveDisplayOrder)
        .setting(AppSettings::ColorNever)

        .arg(Arg::with_name(ARG_ADDR)
            .value_name("ADDRESS:PORT")
            .required(false)
            .default_value(&*DEFAULT_ADDR)
            .help("Binds the server to given address")
            .long_help(concat!(
                "The address and/or port for the server to listen on.\n\n",
                "This argument can be an IP address of a network interface, ",
                "optionally followed by colon and a port number. ",
                "Alternatively, a colon and port alone is also allowed, ",
                "in which case the server will listen on all network interfaces.")))

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


#[cfg(test)]
mod tests {
    use spectral::prelude::*;
    use ::NAME;
    use super::parse_from_argv;

    #[test]
    fn no_args() {
        assert_that!(parse_from_argv(Vec::<&str>::new())).is_ok();
        assert_that!(parse_from_argv(vec![*NAME])).is_ok();
    }

    #[test]
    fn verbosity_args() {
        assert_that!(parse_from_argv(vec![*NAME, "-v"])).is_ok();
        assert_that!(parse_from_argv(vec![*NAME, "-v", "-v"])).is_ok();
        assert_that!(parse_from_argv(vec![*NAME, "-vv"])).is_ok();
        assert_that!(parse_from_argv(vec![*NAME, "-q"])).is_ok();
        // -v & -q are contradictory
        assert_that!(parse_from_argv(vec![*NAME, "-q", "-v"])).is_err();
    }

    #[test]
    fn address_arg() {
        assert_that!(parse_from_argv(vec![*NAME, ":"])).is_err();
        // IP addresses alone are fine.
        assert_that!(parse_from_argv(vec![*NAME, "127.0.0.1"])).is_ok();
        // FIXME: assert_that!(parse_from_argv(vec![*NAME, "0::1"])).is_ok();
        // Port alone is fine, with colon.
        assert_that!(parse_from_argv(vec![*NAME, ":1234"])).is_ok();
        assert_that!(parse_from_argv(vec![*NAME, ":31337"])).is_ok();
        // Both are fine.
        assert_that!(parse_from_argv(vec![*NAME, "127.0.0.1:2345"])).is_ok();
        // FIXME: assert_that!(parse_from_argv(vec![*NAME, "0::1:2345"])).is_ok();
        // FIXME: assert_that!(parse_from_argv(vec![*NAME, "::1:2345"])).is_ok();
        // Invalid port.
        assert_that!(parse_from_argv(vec![*NAME, "4242"])).is_err();  // need colon
        assert_that!(parse_from_argv(vec![*NAME, ":123456789"])).is_err();  // >65536
    }
}
