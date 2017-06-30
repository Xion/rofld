//!
//! roflsh -- Lulz in the shell
//!

             extern crate ansi_term;
#[macro_use] extern crate clap;
             extern crate conv;
#[macro_use] extern crate enum_derive;
             extern crate exitcode;
             extern crate isatty;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate macro_attr;
#[macro_use] extern crate maplit;
#[macro_use] extern crate nom;
             extern crate rofl;
#[macro_use] extern crate slog;
             extern crate slog_envlogger;
             extern crate slog_stdlog;
             extern crate slog_stream;
             extern crate time;

// `log` must be at the end of these declarations because we want to simultaneously:
// * use the standard `log` macros (which would be shadowed by `slog` or even `nom`)
// * be able to initialize the slog logger using slog macros like o!()
#[macro_use] extern crate log;


mod args;
mod logging;


use std::env;
use std::io::{self, Write};
use std::fs;
use std::process::exit;

use args::ArgsError;


lazy_static! {
    /// Application / package name, as filled out by Cargo.
    static ref NAME: &'static str = option_env!("CARGO_PKG_NAME").unwrap_or("roflsh");

    /// Application version, as filled out by Cargo.
    static ref VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
}


fn main() {
    let opts = args::parse().unwrap_or_else(|e| {
        print_args_error(e).unwrap();
        exit(exitcode::USAGE);
    });

    logging::init(opts.verbosity).unwrap();
    if cfg!(debug_assertions) {
        warn!("Debug mode! The program will likely be much slower.");
    }
    for (i, arg) in env::args().enumerate() {
        debug!("argv[{}] = {:?}", i, arg);
    }
    trace!("Options parsed from argv:\n{:#?}", opts);

    match opts.output_path.as_ref() {
        Some(path) => {
            trace!("Opening --output_path file {}...", path.display());
            let file = fs::OpenOptions::new()
                .create(true).write(true).append(false)
                .open(path).unwrap_or_else(|e| {
                    error!("Failed to open output file {} for writing: {}", path.display(), e);
                    exit(exitcode::CANTCREAT);
                });
            debug!("File {} opened successfully", path.display());
            render(opts.image_macro, file)
        }
        None => {
            trace!("No --output_path given, using standard output");
            if isatty::stdout_isatty() {
                warn!("Standard output is a terminal.");
                // TODO: ask for confirmation since this can screw user's terminal
            }
            render(opts.image_macro, io::stdout())
        }
    }.unwrap_or_else(|e| {
        error!("Error while rendering image macro: {}", e);
        exit(exitcode::UNAVAILABLE);
    });
}

/// Print an error that may occur while parsing arguments.
fn print_args_error(e: ArgsError) -> io::Result<()> {
    match e {
        ArgsError::Parse(ref e) =>
            // In case of generic parse error,
            // message provided by the clap library will be the usage string.
            writeln!(&mut io::stderr(), "{}", e.message),
        e => {
            writeln!(&mut io::stderr(), "Failed to parse arguments: {}", e)
        },
    }
}


/// Render given `ImageMacro` and write it to the output.
fn render<W: Write>(im: rofl::ImageMacro, mut output: W) -> io::Result<()> {
    trace!("Rendering macro {:#?}", im);

    // TODO: allow to adjust the resource directories from command line
    let engine = rofl::EngineBuilder::new()
        .template_directory("data/templates")
        .font_directory("data/fonts")
        .build().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let captioned = engine.caption(im)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    trace!("Writing {} bytes to --output_path...", captioned.len());
    output.write_all(captioned.bytes())
}
