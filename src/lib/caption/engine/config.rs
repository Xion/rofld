//! Module with captioning engine configuration.


/// Structure holding configuration for the `Engine`.
///
/// This is shared with `CaptionTask`s.
#[derive(Clone, Copy, Debug)]
pub(in caption) struct Config {
    /// Quality of the generated JPEG images (in %).
    pub jpeg_quality: u8,
}

impl Default for Config {
    /// Initialize Config with default values.
    fn default() -> Self {
        Config {
            jpeg_quality: 85,
        }
    }
}

