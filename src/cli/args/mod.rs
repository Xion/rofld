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


#[cfg(test)]
mod tests {
    use rofl::{HAlign, VAlign};
    use spectral::prelude::*;
    use super::parse_from_argv;
    use ::NAME;

    #[test]
    fn no_args() {
        assert_that!(parse_from_argv(Vec::<&str>::new())).is_err();
        assert_that!(parse_from_argv(vec![*NAME])).is_err();
    }

    #[test]
    fn macro_just_template() {
        let opts = parse_from_argv(vec![*NAME, "zoidberg{Test}"]).unwrap();
        assert_eq!("zoidberg", opts.image_macro.template);
        assert_eq!("Test", opts.image_macro.captions[0].text);
    }

    #[test]
    fn macro_one_text() {
        let opts = parse_from_argv(vec![*NAME, "zoidberg{Test}"]).unwrap();
        assert_eq!("zoidberg", opts.image_macro.template);
        let caption = &opts.image_macro.captions[0];
        assert_eq!("Test", caption.text);
        assert_eq!(VAlign::Bottom, caption.valign);
        assert_eq!(HAlign::Center, caption.halign);
    }

    #[test]
    fn macro_two_texts() {
        let opts = parse_from_argv(vec![*NAME, "zoidberg{Test1}{Test2}"]).unwrap();
        assert_eq!("zoidberg", opts.image_macro.template);

        let caption1 = &opts.image_macro.captions[0];
        let caption2 = &opts.image_macro.captions[1];

        assert_eq!("Test1", caption1.text);
        assert_eq!("Test2", caption2.text);
        // TODO: the valign should be Top+Bottom here but this intelligent valign
        // picking is NYI
        assert_eq!(HAlign::Center, caption1.halign);
        assert_eq!(HAlign::Center, caption2.halign);
    }

    #[test]
    fn macro_text_just_valign() {
        let opts = parse_from_argv(vec![*NAME, "zoidberg{^Test}"]).unwrap();
        assert_eq!("zoidberg", opts.image_macro.template);
        let caption = &opts.image_macro.captions[0];
        assert_eq!("Test", caption.text);
        assert_eq!(VAlign::Top, caption.valign);
    }

    #[test]
    fn macro_text_just_halign() {
        let opts = parse_from_argv(vec![*NAME, "zoidberg{>Test}"]).unwrap();
        assert_eq!("zoidberg", opts.image_macro.template);
        let caption = &opts.image_macro.captions[0];
        assert_eq!("Test", caption.text);
        assert_eq!(HAlign::Right, caption.halign);
    }

    #[test]
    fn macro_text_valign_and_halign() {
        let opts = parse_from_argv(vec![*NAME, "zoidberg{-<Test}"]).unwrap();
        assert_eq!("zoidberg", opts.image_macro.template);
        let caption = &opts.image_macro.captions[0];
        assert_eq!("Test", caption.text);
        assert_eq!(VAlign::Middle, caption.valign);
        assert_eq!(HAlign::Left, caption.halign);
    }

    #[test]
    fn macro_text_halign_and_valign() {
        let opts = parse_from_argv(vec![*NAME, "zoidberg{|^Test}"]).unwrap();
        assert_eq!("zoidberg", opts.image_macro.template);
        let caption = &opts.image_macro.captions[0];
        assert_eq!("Test", caption.text);
        assert_eq!(HAlign::Center, caption.halign);
        assert_eq!(VAlign::Top, caption.valign);
    }

    #[test]
    fn macro_full() {
        let opts = parse_from_argv(vec![
            *NAME, "zoidberg{<^Need a test?}{_>Why not Zoidberg?}"]).unwrap();
        assert_eq!("zoidberg", opts.image_macro.template);

        let caption1 = &opts.image_macro.captions[0];
        assert_eq!("Need a test?", caption1.text);
        assert_eq!(HAlign::Left, caption1.halign);
        assert_eq!(VAlign::Top, caption1.valign);

        let caption2 = &opts.image_macro.captions[1];
        assert_eq!("Why not Zoidberg?", caption2.text);
        assert_eq!(VAlign::Bottom, caption2.valign);
        assert_eq!(HAlign::Right, caption2.halign);
    }

    #[test]
    fn macro_error_no_template() {
        assert_that!(parse_from_argv(vec![*NAME, "{}"])).is_err();
        assert_that!(parse_from_argv(vec![*NAME, "{Test}"])).is_err();
    }

    #[test]
    fn macro_error_unclosed_brace() {
        assert_that!(parse_from_argv(vec![*NAME, "zoidberg{Test"])).is_err();
    }

    #[test]
    fn macro_error_nested_braces() {
        assert_that!(parse_from_argv(vec![*NAME, "zoidberg{Test{More tests}}"]))
            .is_err();
    }

    #[test]
    fn macro_error_closing_brace_first() {
        assert_that!(parse_from_argv(vec![*NAME, "zoidberg}"])).is_err();
    }

    // TODO: test the --json flag (which is actually difficult because it requires mocking
    // or DI'ing or otherwise seeding the stdin with JSON)
}
