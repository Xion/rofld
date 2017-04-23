//! Module for handling command line arguments.

use std::borrow::Cow;
use std::env;
use std::ffi::OsString;
use std::net::{AddrParseError, SocketAddr};
use std::num::ParseIntError;
use std::time::Duration;

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

    /// Number of threads to use for image captioning.
    /// If omitted, the actual count will be based on the number of CPUs.
    pub render_threads: Option<usize>,

    /// Size of the template cache.
    pub template_cache_size: Option<usize>,
    /// Size of the font cache.
    pub font_cache_size: Option<usize>,

    // Maximum time allowed for a single caption request.
    pub request_timeout: Duration,
    // Maximum time the server will wait for pending connections to terminate.
    pub shutdown_timeout: Duration,
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
            let mut addr: Cow<_> = matches.value_of(ARG_ADDR).unwrap().trim().into();

            // If the address is just a port (e.g. ":4242"),
            // then we will prepend it with the default host.
            if addr.starts_with(":") && addr.chars().skip(1).all(|c| c.is_digit(10)) {
                addr = format!("{}{}", DEFAULT_HOST, addr).into();
            }

            // Alternatively, it can be just an interface address, without a port,
            // in which case we'll add the default port.
            let is_just_ipv4 = addr.contains(".") && !addr.contains(":");
            let is_just_ipv6 = addr.starts_with("[") && addr.ends_with("]");
            if is_just_ipv4 || is_just_ipv6 {
                addr = format!("{}:{}", addr, DEFAULT_PORT).into();
            }

            try!(addr.parse())
        };

        let render_threads = match matches.value_of(OPT_RENDER_THREADS) {
            Some(rt) => Some(try!(rt.parse::<usize>().map_err(ArgsError::RenderThreads))),
            None => None,
        };

        let template_cache_size = match matches.value_of(OPT_TEMPLATE_CACHE_SIZE) {
            Some(tcs) => Some(try!(tcs.parse::<usize>().map_err(ArgsError::TemplateCache))),
            None => None,
        };
        let font_cache_size = match matches.value_of(OPT_FONT_CACHE_SIZE) {
            Some(fcs) => Some(try!(fcs.parse::<usize>().map_err(ArgsError::FontCache))),
            None => None,
        };

        let request_timeout = Duration::from_secs(
            try!(matches.value_of(OPT_REQUEST_TIMEOUT).unwrap()
                .parse::<u64>().map_err(ArgsError::RequestTimeout)));
        let shutdown_timeout = Duration::from_secs(
            try!(matches.value_of(OPT_SHUTDOWN_TIMEOUT).unwrap()
                .parse::<u64>().map_err(ArgsError::ShutdownTimeout)));

        Ok(Options{
            verbosity: verbosity,
            address: address,
            render_threads: render_threads,
            template_cache_size: template_cache_size,
            font_cache_size: font_cache_size,
            request_timeout: request_timeout,
            shutdown_timeout: shutdown_timeout,
        })
    }
}

custom_derive! {
    /// Error that can occur while parsing of command line arguments.
    #[derive(Debug,
             Error("command line arguments error"), ErrorDisplay)]
    pub enum ArgsError {
        /// General when parsing the arguments.
        Parse(clap::Error),
        /// Error while parsing the server address.
        Address(AddrParseError),
        /// Error while parsing --render-threads flag.
        RenderThreads(ParseIntError),
        /// Error while parsing --template-cache flag.
        TemplateCache(ParseIntError),
        /// Error while parsing --font-cache flag.
        FontCache(ParseIntError),
        /// Error while parsing --request-timeout flag.
        RequestTimeout(ParseIntError),
        /// Error while parsing --shutdown-timeout flag.
        ShutdownTimeout(ParseIntError),
    }
}
derive_enum_from!(clap::Error => ArgsError::Parse);
derive_enum_from!(AddrParseError => ArgsError::Address);


// Parser configuration

/// Type of the argument parser object
/// (which is called an "App" in clap's silly nomenclature).
type Parser<'p> = clap::App<'p, 'p>;


lazy_static! {
    static ref ABOUT: &'static str = option_env!("CARGO_PKG_DESCRIPTION").unwrap_or("");
}

const ARG_ADDR: &'static str = "address";
const OPT_RENDER_THREADS: &'static str = "render-threads";
const OPT_TEMPLATE_CACHE_SIZE: &'static str = "template-cache";
const OPT_FONT_CACHE_SIZE: &'static str = "font-cache";
const OPT_REQUEST_TIMEOUT: &'static str = "request-timeout";
const OPT_SHUTDOWN_TIMEOUT: &'static str = "shutdown-timeout";
const OPT_VERBOSE: &'static str = "verbose";
const OPT_QUIET: &'static str = "quiet";

const DEFAULT_HOST: &'static str = "0.0.0.0";
const DEFAULT_PORT: u16 = 1337;
const DEFAULT_REQUEST_TIMEOUT: u32 = 10;
const DEFAULT_SHUTDOWN_TIMEOUT: u32 = 30;


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
            .default_value(to_static_str!(format!("{}:{}", DEFAULT_HOST, DEFAULT_PORT)))
            .help("Binds the server to given address")
            .long_help(concat!(
                "The address and/or port for the server to listen on.\n\n",
                "This argument can be an IP address of a network interface, ",
                "optionally followed by colon and a port number. ",
                "Alternatively, a colon and port alone is also allowed, ",
                "in which case the server will listen on all network interfaces.")))

        .arg(Arg::with_name(OPT_RENDER_THREADS)
            .long("render-threads")
            .value_name("N")
            .required(false)
            .help("Number of render threads to use")
            .long_help(concat!(
                "Number of threads used for image captioning.\n\n",
                "If omitted, one thread per each CPU core will be used.")))

        // Cache capacity.
        .arg(Arg::with_name(OPT_TEMPLATE_CACHE_SIZE)
            .long("template-cache")
            .value_name("SIZE")
            .required(false)
            .help("Size of the template cache"))
        .arg(Arg::with_name(OPT_FONT_CACHE_SIZE)
            .long("font-cache")
            .value_name("SIZE")
            .required(false)
            .help("Size of the font cache"))

        // Timeout flags.
        .arg(Arg::with_name(OPT_REQUEST_TIMEOUT)
            .long("request-timeout")
            .value_name("SECS")
            .required(false)
            .default_value(to_static_str!(DEFAULT_REQUEST_TIMEOUT))
            .help("Maximum time allowed for a single request (secs)"))
        .arg(Arg::with_name(OPT_SHUTDOWN_TIMEOUT)
            .long("shutdown-timeout")
            .value_name("SECS")
            .required(false)
            .default_value(to_static_str!(DEFAULT_SHUTDOWN_TIMEOUT))
            .help("Time to wait for remaining connections during shutdown (secs)"))

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
        assert_that!(parse_from_argv(vec![*NAME, "[0::1]"])).is_ok();
        // Port alone is fine, with colon.
        assert_that!(parse_from_argv(vec![*NAME, ":1234"])).is_ok();
        assert_that!(parse_from_argv(vec![*NAME, ":31337"])).is_ok();
        // Both are fine.
        assert_that!(parse_from_argv(vec![*NAME, "127.0.0.1:2345"])).is_ok();
        assert_that!(parse_from_argv(vec![*NAME, "[0::1]:2345"])).is_ok();
        assert_that!(parse_from_argv(vec![*NAME, "[::1]:2345"])).is_ok();
        // Invalid address.
        assert_that!(parse_from_argv(vec![*NAME, "0.0.1"])).is_err();
        assert_that!(parse_from_argv(vec![*NAME, "[::1"])).is_err();
        assert_that!(parse_from_argv(vec![*NAME, "127.0.0.1:"])).is_err();
        // Invalid port.
        assert_that!(parse_from_argv(vec![*NAME, "4242"])).is_err();  // need colon
        assert_that!(parse_from_argv(vec![*NAME, ":123456789"])).is_err();  // >65536
    }

    #[test]
    fn render_threads_arg() {
        // Needs a value.
        assert_that!(parse_from_argv(vec![*NAME, "--render-threads"])).is_err();
        // Value must be a number.
        assert_that!(parse_from_argv(vec![*NAME, "--render-threads", "foo"])).is_err();
        // A positive number.
        assert_that!(parse_from_argv(vec![*NAME, "--render-threads", "-42"])).is_err();
        // This is fine.
        assert_that!(parse_from_argv(vec![*NAME, "--render-threads", "16"])).is_ok();
    }

    #[test]
    fn template_cache_arg() {
        // Needs a value.
        assert_that!(parse_from_argv(vec![*NAME, "--template-cache"])).is_err();
        // Value must be a number.
        assert_that!(parse_from_argv(vec![*NAME, "--template-cache", "foo"])).is_err();
        // A positive number.
        assert_that!(parse_from_argv(vec![*NAME, "--template-cache", "-42"])).is_err();
        // This is fine.
        assert_that!(parse_from_argv(vec![*NAME, "--template-cache", "16"])).is_ok();
    }

    #[test]
    fn font_cache_arg() {
        // Needs a value.
        assert_that!(parse_from_argv(vec![*NAME, "--font-cache"])).is_err();
        // Value must be a number.
        assert_that!(parse_from_argv(vec![*NAME, "--font-cache", "foo"])).is_err();
        // A positive number.
        assert_that!(parse_from_argv(vec![*NAME, "--font-cache", "-42"])).is_err();
        // This is fine.
        assert_that!(parse_from_argv(vec![*NAME, "--font-cache", "16"])).is_ok();
    }
}
