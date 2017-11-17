//! Module defining the model types.

mod align;
mod caption;
mod color;
mod image_macro;
mod size;

pub use self::align::{HAlign, VAlign};
pub use self::caption::{Caption,
                        CaptionBuilder,
                        Error as CaptionBuildError};
pub use self::color::Color;
pub use self::image_macro::{ImageMacro,
                            Builder as ImageMacroBuilder,
                            Error as ImageMacroBuildError};
pub use self::size::Size;
