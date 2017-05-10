//! Module defining constants relevant to the data model.

use super::types::{Color, HAlign};


/// Maximum number of captions an ImageMacro can have.
pub const MAX_CAPTION_COUNT: usize = 16;

/// Name of the default font.
pub const DEFAULT_FONT: &'static str = "Impact";

/// Default color of the text.
pub const DEFAULT_COLOR: Color = Color(0xff, 0xff, 0xff);
/// Default color of the text outline.
/// This should be the inversion of DEFAULT_COLOR.
pub const DEFAULT_OUTLINE_COLOR: Color = Color(0x0, 0x0, 0x0);

/// Default horizontal alignment of text.
pub const DEFAULT_HALIGN: HAlign = HAlign::Center;
