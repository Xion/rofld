//!
//! roflsh -- Lulz in the shell
//!

#[macro_use] extern crate clap;
             extern crate conv;
#[macro_use] extern crate derive_error;
             extern crate exitcode;
#[macro_use] extern crate lazy_static;
             extern crate rofl;


mod args;


use std::io::{self, Write};
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

    // TODO: do stuff actually
    println!("Got options: {:#?}", opts)
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
