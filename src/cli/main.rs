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
             extern crate serde_json;
#[macro_use] extern crate slog;
             extern crate slog_envlogger;
             extern crate slog_stdlog;
             extern crate slog_stream;
             extern crate time;

// `log` must be at the end of these declarations because we want to simultaneously:
// * use the standard `log` macros (which would be shadowed by `slog` or even `nom`)
// * be able to initialize the slog logger using slog macros like o!()
#[macro_use] extern crate log;


#[cfg(test)] #[macro_use] extern crate spectral;


mod args;
mod logging;


use std::env;
use std::io::{self, Write};
use std::fs;
use std::process::exit;

use ansi_term::Colour;
use exitcode::ExitCode;

use args::{ArgsError, Options};


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

    let exit_code = run(opts);
    exit(exit_code)
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

/// Run the application with given options.
fn run(opts: Options) -> ExitCode {
    let result = match opts.output_path.as_ref() {
        Some(path) => {
            trace!("Opening --output_path file {}...", path.display());
            let file = fs::OpenOptions::new()
                .create(true).write(true).append(false)
                .open(path);
            match file {
                Ok(file) => {
                    debug!("File {} opened successfully", path.display());
                    render(opts.image_macro, file)
                }
                Err(e) => {
                    error!("Failed to open output file {} for writing: {}",
                        path.display(), e);
                    return exitcode::CANTCREAT;
                }
            }
        }
        None => {
            trace!("No --output_path given, using standard output");
            if isatty::stdout_isatty() {
                warn!("Standard output is a terminal.");
                let should_continue = ask_before_stdout().unwrap();
                if !should_continue {
                    debug!("User didn't want to print to stdout after all.");
                    return exitcode::OK;
                }
            }
            render(opts.image_macro, io::stdout())
        }
    };

    match result {
        Ok(_) => exitcode::OK,
        Err(e) => {
            error!("Error while rendering image macro: {}", e);
            exitcode::UNAVAILABLE
        }
    }
}


/// Ask the user before printing out binary stuff to stdout.
fn ask_before_stdout() -> io::Result<bool> {
    write!(&mut io::stderr(), "{}", format_stdout_ack_prompt())?;
    let mut answer = String::with_capacity(YES.len());
    io::stdin().read_line(&mut answer)?;
    Ok(answer.trim().to_lowercase() == YES)
}

/// Return the formatted prompt for stdout warning acknowledgment.
fn format_stdout_ack_prompt() -> String {
    const ACK_PROMPT: &'static str =
        "Do you wish to print the binary image output on standard output?";
    if cfg!(unix) {
        format!("{} [{}/{}]: ", ACK_PROMPT, YES, Colour::Green.paint("N"))
    } else {
        format!("{} [{}/{}]: ", ACK_PROMPT, YES, "N")
    }
}

const YES: &'static str = "y";


/// Render given `ImageMacro` and write it to the output.
fn render<W: Write>(im: rofl::ImageMacro, mut output: W) -> io::Result<()> {
    trace!("Rendering macro {:#?}", im);

    // TODO: allow to adjust the resource directories from the command line
    let engine = rofl::EngineBuilder::new()
        .template_directory("data/templates")
        .font_directory("data/fonts")
        .build().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let captioned = engine.caption(im)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    trace!("Writing {} bytes to the output...", captioned.len());
    output.write_all(captioned.bytes())
}
