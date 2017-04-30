//! Module defining constants relevant to the data model.

use super::types::{Color, HAlign};


/// Name of the default font.
pub const DEFAULT_FONT: &'static str = "Impact";

/// Default color of the text.
pub const DEFAULT_COLOR: Color = Color(0xff, 0xff, 0xff);

/// Default horizontal alignment of text.
pub const DEFAULT_HALIGN: HAlign = HAlign::Center;
