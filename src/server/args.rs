//! Module for handling command line arguments.

use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::mem;
use std::net::{AddrParseError, SocketAddr};
use std::num::ParseIntError;
use std::slice;
use std::str;
use std::time::Duration;

use clap::{self, AppSettings, Arg, ArgMatches};
use conv::TryFrom;
use conv::errors::{RangeError, Unrepresentable};
use enum_set::{CLike, EnumSet};

use super::{NAME, VERSION};


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
    /// Quality of GIF animations rendered by the server.
    pub gif_quality: Option<u8>,
    /// Quality of JPEG images produced.
    pub jpeg_quality: Option<u8>,

    /// Size of the template cache.
    pub template_cache_size: Option<usize>,
    /// Size of the font cache.
    pub font_cache_size: Option<usize>,
    /// Which kinds of resources to preload.
    pub preload: EnumSet<Resource>,

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
        let gif_quality = match matches.value_of(OPT_GIF_QUALITY) {
            Some(q) => Some(try!(parse_quality(q).map_err(ArgsError::GifQuality))),
            None => None,
        };
        let jpeg_quality = match matches.value_of(OPT_JPEG_QUALITY) {
            Some(q) => Some(try!(parse_quality(q).map_err(ArgsError::JpegQuality))),
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
        let preload = {
            let mut p = EnumSet::new();
            let (mut all_count, mut none_count) = (0, 0);

            // See what --preload arguments we've got.
            let values = matches.values_of(OPT_PRELOAD)
                .map(|vs| vs.collect()).unwrap_or_else(Vec::new);
            for &v in &values {
                match v {
                    "all" | "both" => { all_count += 1; }
                    "none" => { none_count += 1; }
                    v => { p.insert(Resource::try_from(v).map_err(PreloadError::InvalidResource)?); }
                }
            }

            // Validate the usage of "all" and "none".
            if all_count > 0 && none_count > 0 {
                return Err(ArgsError::Preload(PreloadError::Conflict(
                    "cannot specify `--preload all` and `--preload none` simultaneously".into())));
            }
            if all_count > 0 && all_count < values.len() {
                return Err(ArgsError::Preload(PreloadError::Conflict(
                    "cannot specify `--preload all` alongside specific resource types".into())));
            }
            if none_count > 0 && none_count < values.len() {
                return Err(ArgsError::Preload(PreloadError::Conflict(
                    "cannot specify `--preload none` alongside specific resource types".into())));
            }

            // Apply them if necessary.
            if all_count > 0  { p.extend(Resource::iter_variants()); }
            if none_count > 0 { p.clear(); }
            p
        };

        let request_timeout = Duration::from_secs(
            try!(matches.value_of(OPT_REQUEST_TIMEOUT).unwrap()
                .parse::<u64>().map_err(ArgsError::RequestTimeout)));
        let shutdown_timeout = Duration::from_secs(
            try!(matches.value_of(OPT_SHUTDOWN_TIMEOUT).unwrap()
                .parse::<u64>().map_err(ArgsError::ShutdownTimeout)));

        Ok(Options{
            verbosity, address,
            render_threads, gif_quality, jpeg_quality,
            template_cache_size, font_cache_size, preload,
            request_timeout, shutdown_timeout,
        })
    }
}

/// Parse a string into an image quality percentage.
fn parse_quality(s: &str) -> Result<u8, QualityError> {
    match s.parse()? {
        q if q <= 0 => Err(RangeError::NegOverflow(q).into()),
        q if q > 100 => Err(RangeError::PosOverflow(q).into()),
        q => Ok(q),
    }
}


/// Error that can occur while parsing of command line arguments.
#[derive(Debug, Error)]
pub enum ArgsError {
    /// General when parsing the arguments.
    Parse(clap::Error),
    /// Error while parsing the server address.
    Address(AddrParseError),
    /// Error while parsing --render-threads flag.
    #[error(no_from)]
    RenderThreads(ParseIntError),
    /// Error while parsing --gif-quality flag.
    #[error(no_from)]
    GifQuality(QualityError),
    /// Error while parsing --jpeg-quality flag.
    #[error(no_from)]
    JpegQuality(QualityError),
    /// Error while parsing --template-cache flag.
    #[error(no_from)]
    TemplateCache(ParseIntError),
    /// Error while parsing --font-cache flag.
    #[error(no_from)]
    FontCache(ParseIntError),
    /// Error while parsing --preload flag.
    Preload(PreloadError),
    /// Error while parsing --request-timeout flag.
    #[error(no_from)]
    RequestTimeout(ParseIntError),
    /// Error while parsing --shutdown-timeout flag.
    #[error(no_from)]
    ShutdownTimeout(ParseIntError),
}

macro_attr! {
    /// Error that can occur while parsing the --preload flag.
    #[derive(Debug, EnumFromInner!)]
    pub enum PreloadError {
        /// "all" or "none" is used alongside other options.
        Conflict(Box<Error>),
        /// Unknown resource type.
        InvalidResource(Unrepresentable<String>),
    }
}
impl Error for PreloadError {
    fn description(&self) -> &str { "invalid --preload value" }
    fn cause(&self) -> Option<&Error> {
        match self {
            &PreloadError::Conflict(ref e) => Some(&**e),
            &PreloadError::InvalidResource(ref e) => Some(e),
        }
    }
}
impl fmt::Display for PreloadError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}: {}", self.description(), match *self {
            PreloadError::Conflict(ref e) => format!("{}", e),
            PreloadError::InvalidResource(
                Unrepresentable(ref s)) => format!("unknown resource type `{}`", s),
        })
    }
}

/// Error that can occur while parsing an --X-quality flag.
#[derive(Debug, Error)]
pub enum QualityError {
    /// Error while parsing the value as number.
    Parse(ParseIntError),
    /// Error for when the quality value is out of range.
    Range(RangeError<u8>),
}


macro_attr! {
    /// One of the resources used for rendering image macros.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash,
             IterVariants!(Resources))]
    #[repr(u32)]
    pub enum Resource { Template, Font }
}
impl CLike for Resource {
    fn to_u32(&self) -> u32            { *self as u32 }
    unsafe fn from_u32(v: u32) -> Self { mem::transmute(v) }
}
impl<'s> TryFrom<&'s str> for Resource {
    type Err = Unrepresentable<String>;

    fn try_from(s: &'s str) -> Result<Self, Self::Err> {
        let s = s.trim_right_matches("s");  // accept singular/plural
        for r in Resource::iter_variants() {
            if r.to_string().trim_right_matches("s") == s {
                return Ok(r);
            }
        }
        Err(Unrepresentable(s.to_owned()))
    }
}
impl ToString for Resource {
    fn to_string(&self) -> String {
        format!("{:?}s", self).to_lowercase()
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
const OPT_RENDER_THREADS: &'static str = "render-threads";
const OPT_GIF_QUALITY: &'static str = "gif-quality";
const OPT_JPEG_QUALITY: &'static str = "jpeg-quality";
const OPT_TEMPLATE_CACHE_SIZE: &'static str = "template-cache";
const OPT_FONT_CACHE_SIZE: &'static str = "font-cache";
const OPT_PRELOAD: &'static str = "preload";
const OPT_REQUEST_TIMEOUT: &'static str = "request-timeout";
const OPT_SHUTDOWN_TIMEOUT: &'static str = "shutdown-timeout";
const OPT_VERBOSE: &'static str = "verbose";
const OPT_QUIET: &'static str = "quiet";

const VALID_PRELOAD: &'static [&'static str] = &["all", "both", "none",
                                                 "templates", "fonts"];

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

        .setting(AppSettings::StrictUtf8)

        .setting(AppSettings::UnifiedHelpMessage)
        .setting(AppSettings::DontCollapseArgsInUsage)
        .setting(AppSettings::DeriveDisplayOrder)
        .setting(AppSettings::ColorNever)

        .arg(Arg::with_name(ARG_ADDR)
            .value_name("ADDRESS:PORT")
            .required(false)
            .default_value(to_static_str(format!("{}:{}", DEFAULT_HOST, DEFAULT_PORT)))
            .help("Binds the server to given address")
            .long_help(concat!(
                "The address and/or port for the server to listen on.\n\n",
                "This argument can be an IP address of a network interface, ",
                "optionally followed by colon and a port number. ",
                "Alternatively, a colon and port alone is also allowed, ",
                "in which case the server will listen on all network interfaces.")))

        // Rendering options.
        .arg(Arg::with_name(OPT_RENDER_THREADS)
            .long("render-threads")
            .value_name("N")
            .required(false)
            .help("Number of render threads to use")
            .long_help(concat!(
                "Number of threads used for image captioning.\n\n",
                "If omitted, one thread per each CPU core will be used.")))
        .arg(Arg::with_name(OPT_GIF_QUALITY)
            .long("gif-quality")
            .value_name("PERCENT")
            .required(false)
            .help("Quality of GIF animations produced")
            .long_help(concat!(
                "Quality percentage of GIF animations rendered by the server.\n\n",
                "Note that anything higher than 70 is likely to be *very* slow.")))
        .arg(Arg::with_name(OPT_JPEG_QUALITY)
            .long("jpeg-quality")
            .value_name("PERCENT")
            .required(false)
            .help("Quality of JPEG images rendered"))

        // Cache options.
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
        .arg(Arg::with_name(OPT_PRELOAD)
            .long("preload")
            .value_name("WHAT")
            .required(false)
            .possible_values(VALID_PRELOAD)
            .multiple(true).number_of_values(1)
            .help("What resources to preload on server startup")
            .long_help(concat!(
                "Which resource caches should be filled when the server starts\n\n",
                "All the resources found during server startup will be loaded & cached ",
                "(up to the relevant caches' capacities). ",
                "The exact subset of resources to preload in this way is randomized.")))

        // Timeout flags.
        .arg(Arg::with_name(OPT_REQUEST_TIMEOUT)
            .long("request-timeout")
            .value_name("SECS")
            .required(false)
            .default_value(to_static_str(
                // Disable request timeouts in debug mode unless specifically requested.
                if cfg!(debug_assertions) { 0 } else { DEFAULT_REQUEST_TIMEOUT }
            ))
            .help("Maximum time allowed for a single request (secs)"))
        .arg(Arg::with_name(OPT_SHUTDOWN_TIMEOUT)
            .long("shutdown-timeout")
            .value_name("SECS")
            .required(false)
            .default_value(to_static_str(
                // Disable waiting for server to shut down in debug mode by default.
                if cfg!(debug_assertions) { 0 } else { DEFAULT_SHUTDOWN_TIMEOUT }
            ))
            .help("Time to wait for remaining connections during shutdown (secs)"))

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

/// Convert a value to a &'static str by leaking the memory of an owned String.
fn to_static_str<T: ToString>(v: T) -> &'static str {
    let s = v.to_string();
    unsafe {
        let (ptr, len) = (s.as_ptr(), s.len());
        mem::forget(s);
        let bytes: &'static [u8] = slice::from_raw_parts(ptr, len);
        str::from_utf8_unchecked(bytes)
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use conv::TryFrom;
    use spectral::prelude::*;
    use ::NAME;
    use super::{parse_from_argv, Resource, VALID_PRELOAD};

    /// Check that the VALID_PRELOAD options make sense.
    #[test]
    fn preload_is_consistent() {
        let mut valid_preload_opts: HashSet<_> = VALID_PRELOAD.iter().collect();
        let resources = Resource::iter_variants().collect::<HashSet<_>>();

        // Remove the broad / special values for the --preload flag.
        for x in ["all", "none"].iter() {
            assert!(valid_preload_opts.contains(x),
                "{:?} does not contain {:?}", VALID_PRELOAD, x);
            valid_preload_opts.remove(x);
        }
        assert!(valid_preload_opts.remove(&"both") == (resources.len() == 2),
            "{:?} may only contain {:?} iff there are exactly two resource types",
            VALID_PRELOAD, "both");

        // What remains should be convertible to resources -- all resources.
        // (THough it's fine if more than one value converts to the same Resource).
        let mut converted_resources = HashSet::new();
        for &vp in valid_preload_opts {
            let resource = Resource::try_from(vp)
                .expect(&format!("{:?} doesn't convert to a preloadable Resource", vp));
            converted_resources.insert(resource);
        }
        assert_eq!(resources, converted_resources,
            "{:?} doesn't contain values for all Resources", VALID_PRELOAD);
    }

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
    fn gif_quality_arg() {
        // Needs a value.
        assert_that!(parse_from_argv(vec![*NAME, "--gif-quality"])).is_err();
        // Value must be a number.
        assert_that!(parse_from_argv(vec![*NAME, "--gif-quality", "foo"])).is_err();
        // A positive number.
        assert_that!(parse_from_argv(vec![*NAME, "--gif-quality", "-42"])).is_err();
        // Within range.
        assert_that!(parse_from_argv(vec![*NAME, "--gif-quality", "169"])).is_err();
        // This is fine.
        assert_that!(parse_from_argv(vec![*NAME, "--gif-quality", "65"])).is_ok();
    }

    #[test]
    fn jpeg_quality_arg() {
        // Needs a value.
        assert_that!(parse_from_argv(vec![*NAME, "--jpeg-quality"])).is_err();
        // Value must be a number.
        assert_that!(parse_from_argv(vec![*NAME, "--jpeg-quality", "foo"])).is_err();
        // A positive number.
        assert_that!(parse_from_argv(vec![*NAME, "--jpeg-quality", "-42"])).is_err();
        // Within range.
        assert_that!(parse_from_argv(vec![*NAME, "--jpeg-quality", "169"])).is_err();
        // This is fine.
        assert_that!(parse_from_argv(vec![*NAME, "--jpeg-quality", "65"])).is_ok();
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

    #[test]
    fn preload_arg() {
        // Needs a value.
        assert_that!(parse_from_argv(vec![*NAME, "--preload"])).is_err();
        // Value can be all/none.
        assert_that!(parse_from_argv(vec![*NAME, "--preload", "all"])).is_ok();
        assert_that!(parse_from_argv(vec![*NAME, "--preload", "none"])).is_ok();
        // It can also be a resource type.
        assert_that!(parse_from_argv(vec![*NAME, "--preload", "templates"])).is_ok();
        assert_that!(parse_from_argv(vec![*NAME, "--preload", "fonts"])).is_ok();
        // But not both, since that doesn't make sense.
        assert_that!(parse_from_argv(vec![
            *NAME, "--preload", "templates", "--preload", "all"])).is_err();
        assert_that!(parse_from_argv(vec![
            *NAME, "--preload", "fonts", "--preload", "none"])).is_err();
        // all & none simultaneously makes even less of a sense.
        assert_that!(parse_from_argv(vec![
            *NAME, "--preload", "all", "--preload", "none"])).is_err();
        // Multiple resource types should work though (even if redundant).
        assert_that!(parse_from_argv(vec![
            *NAME, "--preload", "templates", "--preload", "fonts"])).is_ok();
        assert_that!(parse_from_argv(vec![
            *NAME, "--preload", "templates", "--preload", "templates"])).is_ok();
        assert_that!(parse_from_argv(vec![
            *NAME, "--preload", "templates", "--preload", "templates",
            "--preload", "fonts", "--preload", "fonts", "--preload", "fonts"])).is_ok();
    }
}
