//! Tests for deserializing complete ImageMacros from query strings.

use serde_qs::{self, from_str as from_qs};
use spectral::prelude::*;

use model::{Caption, Color, ImageMacro, VAlign};


#[test]
fn simple_captions() {
    let input = "template=zoidberg&top_text=Need%20a%20meme?&bottom_text=Why%20not%20Zoidberg?";
    assert_that!(parse(input)).is_ok().is_equal_to(&*ZOIDBERG);
}

#[test]
fn simple_captions_with_color() {
    let input = "template=fullofstars&\
        top_text=Oh%20my%20god&top_color=0xffff00&\
        bottom_text=It%27s%20full%20of%20colors&bottom_color=0x00ffff";
    assert_that!(parse(input)).is_ok().is_equal_to(&*FULL_OF_COLORS);
}

#[test]
fn full_captions_with_just_text() {
    let input = "template=zoidberg&captions[0]=Need%20a%20meme?&captions[1]=Why%20not%20Zoidberg?";
    assert_that!(parse(input)).is_ok().is_equal_to(&*ZOIDBERG);
}

#[test]
fn full_captions_with_valign() {
    let input = "template=zoidberg&\
        captions[0][valign]=top&captions[0][text]=Need%20a%20meme?&\
        captions[1][valign]=bottom&captions[1][text]=Why%20not%20Zoidberg?";
    assert_that!(parse(input)).is_ok().is_equal_to(&*ZOIDBERG);
}

#[test]
fn full_captions_with_valign_and_color() {
    let input = "template=fullofstars&\
        captions[0][text]=Oh%20my%20god&\
            captions[0][color]=0xffff00&captions[0][valign]=top&\
        captions[1][text]=It%27s%20full%20of%20colors&\
            captions[1][color]=0x00ffff&captions[1][valign]=bottom";
    assert_that!(parse(input)).is_ok().is_equal_to(&*FULL_OF_COLORS);
}

#[test]
fn caption_text_with_ampersand() {
    // The ampersand is of course URL-encoded (as %26).
    let input = "template=zoidberg?top_text=Need%20a%20meme%20%26%20text?";
    assert_that!(parse(input)).is_ok();
}


// Common test data

lazy_static! {
    static ref ZOIDBERG: ImageMacro = ImageMacro{
        template: "zoidberg".into(),
        captions: vec![
            Caption{
                text: "Need a meme?".into(),
                ..Caption::at(VAlign::Top)
            },
            Caption{
                text: "Why not Zoidberg?".into(),
                ..Caption::at(VAlign::Bottom)
            },
        ],
        ..Default::default()
    };

    static ref FULL_OF_COLORS: ImageMacro = ImageMacro{
        template: "fullofstars".into(),
        captions: vec![
            Caption{
                text: "Oh my god".into(),
                color: Color(255, 255, 0),
                ..Caption::at(VAlign::Top)
            },
            Caption{
                text: "It's full of colors".into(),
                color: Color(0, 255, 255),
                ..Caption::at(VAlign::Bottom)
            },
        ],
        ..Default::default()
    };
}


// Utility functions

fn parse(qs: &str) -> Result<ImageMacro, serde_qs::Error> {
    // This function may seem pointless, but it saves us on using turbofish everywhere
    // to tell the compiler it's ImageMacro we're deserializing.
    from_qs(qs)
}
