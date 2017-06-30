//! Module handling the command line argument
//! that specifies the image macro to render.

use std::error;
use std::fmt;

use nom::{alphanumeric, IResult, Needed};

use rofl::{Caption, CaptionBuilder, HAlign, ImageMacro, ImageMacroBuilder, VAlign};


/// Parse a MACRO command line argument into an `ImageMacro`.
pub fn parse(s: &str) -> Result<ImageMacro, Error> {
    match root(s) {
        IResult::Done(remaining, im) => {
            if remaining.is_empty() {
                Ok(im)
            } else {
                Err(Error::Excess(remaining.len()))
            }
        },
        IResult::Incomplete(needed) => {
            let expected = match needed {
                Needed::Unknown => None,
                Needed::Size(n) => Some(n),
            };
            Err(Error::Incomplete(expected))
        }
        IResult::Error(_) => Err(Error::Parse),
    }
}


/// Error that can occur when parsing image macro specification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// Erorr during parsing.
    // TODO: better error here
    Parse,
    /// Error for when we expected this many bytes of more input than we've got.
    Incomplete(Option<usize>),
    /// Error for when there is still some input left after parsing has finished.
    Excess(usize),
}

impl error::Error for Error {
    fn description(&self) -> &str { "invalid image macro definition" }
    fn cause(&self) -> Option<&error::Error> { None }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Parse => write!(fmt, "parse error"),
            Error::Incomplete(needed) =>
                write!(fmt, "incomplete input ({} needed)", match needed {
                    Some(n) => format!("{} byte(s)", n),
                    None => format!("undetermined number of bytes"),
                }),
            Error::Excess(n) => write!(fmt, "excess {} byte(s) found in input", n),
        }
    }
}


// Syntax definition

/// Root of the parser hierarchy.
/// Parses the entire `ImageMacro`.
named!(root(&str) -> ImageMacro, do_parse!(
    opt!(tag_s!("\\")) >>
    template: alphanumeric >>
    captions: many0!(caption) >>
    ({
        let mut builder = ImageMacroBuilder::new()
            .template(template);
        for cap in captions {
            builder = builder.caption(cap);
        }
        builder.build().unwrap() // TODO: error handling
    })
));

/// Parse a single `Caption`.
named!(caption(&str) -> Caption, do_parse!(
    tag_s!("{") >>
    valign: opt!(valign) >> halign: opt!(halign) >>
    text: take_until_s!("}") >>  // TODO: escaping of } so it can be included in text
    tag_s!("}") >>
    ({
        let mut builder = CaptionBuilder::new()
            .valign(valign.unwrap_or(VAlign::Bottom));
            // TODO: determine valign (if not given) based on number of captions
        if let Some(halign) = halign {
            builder = builder.halign(halign);
        }
        if !text.is_empty() {
            builder = builder.text(text.to_owned());
        }
        builder.build().unwrap()
    })
));

/// Parse the vertical alignment character marker.
named!(valign(&str) -> VAlign, alt!(
    tag_s!("^") => { |_| VAlign::Top } |
    tag_s!("-") => { |_| VAlign::Middle } |
    tag_s!("_") => { |_| VAlign::Bottom }
));

/// Parse the horizontal alignment character marker.
named!(halign(&str) -> HAlign, alt!(
    tag_s!("<") => { |_| HAlign::Left } |
    tag_s!("|") => { |_| HAlign::Center } |
    tag_s!(">") => { |_| HAlign::Right }
));
