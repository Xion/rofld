//! Tests for deserializing ImageMacros.

use serde_json::{self, from_value as from_json, Value};
use serde_qs::{self, from_str as from_qs};
use spectral::prelude::*;

use super::super::{Caption, Color, HAlign, ImageMacro, VAlign};


#[test]
fn just_template() {
    let input = json!({"template": "zoidberg"});
    let expected = ImageMacro{
        template: "zoidberg".into(),
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn scaled_template() {
    let input = json!({
        "template": "zoidberg",
        "width": 640,
        "height": 480,
    });
    let expected = ImageMacro{
        template: "zoidberg".into(),
        width: Some(640),
        height: Some(480),
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn one_simple_caption() {
    let input = json!({
        "template": "grumpycat",
        "bottom_text": "No.",
    });
    let expected = ImageMacro{
        template: "grumpycat".into(),
        captions: vec![
            Caption{
                text: "No.".into(),
                ..Caption::at(VAlign::Bottom)
            },
        ],
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn several_simple_captions() {
    let input = json!({
        "template": "zoidberg",
        "top_text": "Need more text?",
        "bottom_text": "Why not Zoidberg?",
    });
    let expected = ImageMacro{
        template: "zoidberg".into(),
        captions: vec![
            Caption{
                text: "Need more text?".into(),
                ..Caption::at(VAlign::Top)
            },
            Caption{
                text: "Why not Zoidberg?".into(),
                ..Caption::at(VAlign::Bottom)
            },
        ],
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn simple_caption_with_invalid_alignment() {
    let input = json!({
        "template": "firstworldproblems",
        "top_text": "My meme text",
        "bottom_text": "is not aligned correctly",
        "bottom_align": "justify",
    });
    assert_that!(parse_json(input)).is_err().map(|e| {
        let msg = format!("{}", e);
        assert_that!(msg).contains("unknown variant");
        assert_that!(msg).contains("justify");
        e
    });
}

#[test]
fn simple_captions_with_alignment() {
    let input = json!({
        "template": "doge",
        "top_text": "much aligned",
        "top_align": "left",
        "middle_text": "very text",
        "middle_align": "right",
        "bottom_text": "wow",
        "bottom_align": "center",
    });
    let expected = ImageMacro{
        template: "doge".into(),
        captions: vec![
            Caption{
                text: "much aligned".into(),
                halign: HAlign::Left,
                ..Caption::at(VAlign::Top)
            },
            Caption{
                text: "very text".into(),
                halign: HAlign::Right,
                ..Caption::at(VAlign::Middle)
            },
            Caption{
                text: "wow".into(),
                halign: HAlign::Center,
                ..Caption::at(VAlign::Bottom)
            },
        ],
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn simple_captions_with_color() {
    let input = json!({
        "template": "doge",
        "top_text": "very color",
        "top_color": "red",
        "middle_text": "much rgb",
        "middle_color": "rgb(0,255,255)",
        "bottom_text": "wow",
        "bottom_color": "lime",
    });
    let expected = ImageMacro{
        template: "doge".into(),
        captions: vec![
            Caption{
                text: "very color".into(),
                color: Color(0xff, 0, 0),
                ..Caption::at(VAlign::Top)
            },
            Caption{
                text: "much rgb".into(),
                color: Color(0, 0xff, 0xff),
                ..Caption::at(VAlign::Middle)
            },
            Caption{
                text: "wow".into(),
                color: Color(0, 0xff, 0),
                ..Caption::at(VAlign::Bottom)
            },
        ],
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn simple_captions_without_outline() {
    let input = json!({
        "template": "grumpycat",
        "top_text": "Outline?",
        "top_outline": null,
        "bottom_text": "No.",
        "bottom_outline": null,
    });
    let expected = ImageMacro{
        template: "grumpycat".into(),
        captions: vec![
            Caption{
                text: "Outline?".into(),
                outline: None,
                ..Caption::at(VAlign::Top)
            },
            Caption{
                text: "No.".into(),
                outline: None,
                ..Caption::at(VAlign::Bottom)
            },
        ],
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn custom_font_for_simple_captions() {
    let input = json!({
        "template": "grumpycat",
        "font": "Comic Sans",
        "top_text": "No.",
        "bottom_text": "Just no.",
    });
    let expected = ImageMacro{
        template: "grumpycat".into(),
        captions: vec![
            Caption{
                text: "No.".into(),
                font: "Comic Sans".into(),
                ..Caption::at(VAlign::Top)
            },
            Caption{
                text: "Just no.".into(),
                font: "Comic Sans".into(),
                ..Caption::at(VAlign::Bottom)
            },
        ],
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn custom_color_for_simple_captions() {
    let input = json!({
        "template": "boromir",
        "color": "black",
        "top_text": "One does not simply",
        "bottom_text": "make a meme",
    });
    let expected = ImageMacro{
        template: "boromir".into(),
        captions: vec![
            Caption{
                text: "One does not simply".into(),
                color: Color(0, 0, 0),
                ..Caption::at(VAlign::Top)
            },
            Caption{
                text: "make a meme".into(),
                color: Color(0, 0, 0),
                ..Caption::at(VAlign::Bottom)
            },
        ],
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn no_outline_for_simple_captions() {
    let input = json!({
        "template": "y_u_no",
        "outline": null,
        "top_text": "Y U no",
        "bottom_text": "draw a text border",
    });
    let expected = ImageMacro{
        template: "y_u_no".into(),
        captions: vec![
            Caption{
                text: "Y U no".into(),
                outline: None,
                ..Caption::at(VAlign::Top)
            },
            Caption{
                text: "draw a text border".into(),
                outline: None,
                ..Caption::at(VAlign::Bottom)
            },
        ],
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn custom_outline_for_simple_captions() {
    let input = json!({
        "template": "yodawg",
        "outline": "blue",
        "top_text": "Yo dawg, I heard you like colors",
        "bottom_text": "so I put a colored text in a colored outline",
    });
    let expected = ImageMacro{
        template: "yodawg".into(),
        captions: vec![
            Caption{
                text: "Yo dawg, I heard you like colors".into(),
                outline: Some(Color(0, 0, 0xff)),
                ..Caption::at(VAlign::Top)
            },
            Caption{
                text: "so I put a colored text in a colored outline".into(),
                outline: Some(Color(0, 0, 0xff)),
                ..Caption::at(VAlign::Bottom)
            },
        ],
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn empty_full_captions() {
    let input = json!({
        "template": "anditsgone",
        "captions": [],
    });
    let expected = ImageMacro{
        template: "anditsgone".into(),
        captions: vec![],
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn full_captions_with_just_text() {
    let input = json!({
        "template": "slowpoke",
        "captions": ["Hey guys", "Have you heard about this meme thing?"],
    });
    let expected = ImageMacro{
        template: "slowpoke".into(),
        captions: vec![
            Caption{
                text: "Hey guys".into(),
                ..Caption::at(VAlign::Top)
            },
            Caption{
                text: "Have you heard about this meme thing?".into(),
                ..Caption::at(VAlign::Bottom)
            }
        ],
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn too_many_full_captions_with_just_text() {
    let input = json!({
        "template": "firstworldproblems",
        "captions": [
            "My meme generator",
            "seems to be pretty limited",
            "and it cannot automatically infer",
            "where to put",
            "all those captions",
            "without further hints.",
        ],
    });
    assert_that!(parse_json(input)).is_err().map(|e| {
        let msg = format!("{}", e);
        assert_that!(msg).contains("invalid length");
        for allowed in ["0", "1", "2", "3"].iter() {
            assert_that!(msg).contains(allowed);
        }
        e
    });
}

#[test]
fn full_captions_with_parameters() {
    let input = json!({
        "template": "philosoraptor",
        "captions": [
            {
                "valign": "top",
                "text": "If you communicate with memes",
                "halign": "center",
            },
            {
                "valign": "bottom",
                "text": "is it called comemecation?",
                "halign": "center",
            },
        ],
    });
    let expected = ImageMacro{
        template: "philosoraptor".into(),
        captions: vec![
            Caption{
                text: "If you communicate with memes".into(),
                halign: HAlign::Center,
                ..Caption::at(VAlign::Top)
            },
            Caption{
                text: "is it called comemecation?".into(),
                halign: HAlign::Center,
                ..Caption::at(VAlign::Bottom)
            },
        ],
        ..Default::default()
    };
    assert_that!(parse_json(input)).is_ok().is_equal_to(expected);
}

#[test]
fn mixed_full_captions() {
    let input = json!({
        "template": "asianfather",
        "captions": [
            "Meme with text?",
            {
                "text": "Why not meme with aligned text?",
                "valign": "bottom",
                "halign": "center",
            }
        ],
    });
    assert_that!(parse_json(input)).is_err().map(|e| {
        let msg = format!("{}", e);
        assert_that!(msg).contains("captions");
        assert_that!(msg).contains("must be either");
        assert_that!(msg).contains("or");
        assert_that!(msg).contains("all");
        e
    });
}

#[test]
fn qs_simple_captions() {
    let input = "template=zoidberg&top_text=Need%20a%20meme?&bottom_text=Why%20not%20Zoidberg?";
    assert_that!(parse_qs(input)).is_ok().is_equal_to(&*ZOIDBERG);
}

#[test]
fn qs_simple_captions_with_color() {
    let input = "template=fullofstars&\
        top_text=Oh%20my%20god&top_color=0xffff00&\
        bottom_text=It%27s%20full%20of%20colors&bottom_color=0x00ffff";
    assert_that!(parse_qs(input)).is_ok().is_equal_to(&*FULL_OF_COLORS);
}

#[test]
fn qs_full_captions_with_just_text() {
    let input = "template=zoidberg&captions[0]=Need%20a%20meme?&captions[1]=Why%20not%20Zoidberg?";
    assert_that!(parse_qs(input)).is_ok().is_equal_to(&*ZOIDBERG);
}

#[test]
fn qs_full_captions_with_valign() {
    let input = "template=zoidberg&\
        captions[0][valign]=top&captions[0][text]=Need%20a%20meme?&\
        captions[1][valign]=bottom&captions[1][text]=Why%20not%20Zoidberg?";
    assert_that!(parse_qs(input)).is_ok().is_equal_to(&*ZOIDBERG);
}

#[test]
fn qs_full_captions_with_valign_and_color() {
    let input = "template=fullofstars&\
        captions[0][text]=Oh%20my%20god&\
            captions[0][color]=0xffff00&captions[0][valign]=top&\
        captions[1][text]=It%27s%20full%20of%20colors&\
            captions[1][color]=0x00ffff&captions[1][valign]=bottom";
    assert_that!(parse_qs(input)).is_ok().is_equal_to(&*FULL_OF_COLORS);
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

fn parse_json(json: Value) -> Result<ImageMacro, serde_json::Error> {
    // This function may seem pointless, but it saves us on using turbofish everywhere
    // to tell the compiler it's ImageMacro we're deserializing.
    from_json(json)
}

fn parse_qs(qs: &str) -> Result<ImageMacro, serde_qs::Error> {
    // Ditto.
    from_qs(qs)
}
