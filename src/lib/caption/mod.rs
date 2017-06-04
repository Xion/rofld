//! Module implementing image captioning.

mod engine;
mod error;
mod output;
mod task;


pub use self::engine::Engine;
pub use self::error::CaptionError;
pub use self::output::CaptionOutput;
