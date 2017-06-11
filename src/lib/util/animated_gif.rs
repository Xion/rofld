//! Module handling the decoding & encoding of animated GIFs.
//! This is done by wrapping over the API exposed by several image-related crates.

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::slice;

use color_quant::NeuQuant;
use gif::{self, SetParameter};
use gif_dispose::Screen;
use image::{DynamicImage, GenericImage, RgbaImage};


// Data structures

/// Animation loaded from a GIF file.
/// The frames are kept in their decoded (RGBA) form.
#[derive(Clone)]
pub struct GifAnimation {
    /// Width of the animation canvas (logical screen).
    pub width: u16,
    /// Height of the animation canvas (logical screen).
    pub height: u16,
    /// Global palette (Color Table).
    /// This is a contiguous array of RGB bytes.
    pub palette: Vec<u8>,
    /// Index of the background color in global palette, if any.
    pub bg_color: Option<usize>,
    /// Animation frames.
    frames: Vec<GifFrame>,
}

impl GifAnimation {
    #[inline]
    pub fn frames_count(&self) -> usize {
        self.frames.len()
    }

    #[inline]
    pub fn iter_frames<'a>(&'a self) -> Box<Iterator<Item=&'a GifFrame> + 'a> {
        Box::new(self.frames.iter())
    }
}

impl fmt::Debug for GifAnimation {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let colors = self.palette.len() / RGB_SIZE_BYTES;
        fmt.debug_struct("GifAnimation")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("palette", &format_args!("<{} colors>", colors))
            .field("bg_color", &self.bg_color)
            .field("frames", &format_args!("<{} frames>", self.frames.len()))
            .finish()
    }
}

const RGB_SIZE_BYTES: usize = 3;


/// A single frame of an animated GIF template.
#[derive(Clone)]
pub struct GifFrame {
    /// The image of the frame.
    pub image: DynamicImage,
    /// gif::Frame structure containing just the metadata of the frame.
    /// The actual buffer is emptied and converted into the `image`.
    pub metadata: gif::Frame<'static>,
}

impl GifFrame {
    /// Create a GifFrame from the gif::Frame metadata & specified RGBA buffer.
    pub fn from_rgba<'f>(metadata: &gif::Frame<'f>,
                         width: usize, height: usize, pixels: &[u8]) -> Self {
        let image = DynamicImage::ImageRgba8(
            RgbaImage::from_raw(
                width as u32, height as u32, pixels.to_vec()).unwrap());
        let metadata = gif::Frame{
            buffer: vec![].into(),
            // Copy the rest of the metadata.
            delay: metadata.delay,
            dispose: metadata.dispose,
            transparent: metadata.transparent,
            needs_user_input: metadata.needs_user_input,
            top: metadata.top,
            left: metadata.left,
            width: metadata.width,
            height: metadata.height,
            interlaced: metadata.interlaced,
            palette: metadata.palette.clone(),
        };
        GifFrame{image, metadata}
    }
}

impl fmt::Debug for GifFrame {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let (w, h) = self.image.dimensions();
        fmt.debug_struct("GifFrame")
            .field("image", &format_args!("{}x{}", w, h))
            .field("metadata", &self.metadata)
            .finish()
    }
}


// Checking GIF properties

/// Check if the path points to a GIF file.
pub fn is_gif<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    trace!("Checking if {} is a GIF", path.display());
    path.extension().and_then(|s| s.to_str())
        .map(|ext| ext.to_lowercase() == "gif").unwrap_or(false)
}

/// Check if given GIF image is animated.
/// Returns None if it cannot be determined (e.g. file doesn't exist).
pub fn is_gif_animated<P: AsRef<Path>>(path: P) -> Option<bool> {
    let path = path.as_ref();
    trace!("Checking if {} is an animated GIF", path.display());

    let mut file = try_opt!(File::open(path).map_err(|e| {
        warn!("Failed to open file {} to check if it's animated GIF: {}",
            path.display(), e); e
    }).ok());

    // The `image` crate technically has an ImageDecoder::is_animated() method,
    // but it doesn't seem to actually work.
    // So instead we just check if the GIF has at least two frames spaced in time.

    let mut decoder = gif::Decoder::new(&mut file);
    decoder.set(gif::ColorOutput::Indexed);
    decoder.set(MEMORY_LIMIT);
    let mut reader = try_opt!(decoder.read_info().ok());

    let mut frame_count = 0;
    let mut delay_ms = 0;
    while let Some(frame) = try_opt!(reader.next_frame_info().ok()) {
        frame_count += 1;
        delay_ms += frame.delay * 10;  // GIF delay unit is 10ms.
        if frame_count > 1 && delay_ms > 0 {
            trace!("File {} is a >={}ms animated GIF with {}+ frames",
                path.display(), delay_ms, frame_count);
            return Some(true);
        }
    }

    if frame_count > 0 {
        trace!("File {} is a still but compound GIF image with {} parts",
            path.display(), frame_count);
    } else {
        trace!("File {} is a still GIF image", path.display());
    }
    Some(false)
}

// TODO: make this an Engine configuration parameter
const MEMORY_LIMIT: gif::MemoryLimit = gif::MemoryLimit(32 * 1024 * 1024);


// Decoding animated GIFs

/// Error that can occur while decoding animated GIF.
#[derive(Debug)]
pub enum DecodeError {
    /// I/O error encountered when decoding GIF.
    Io(io::Error),
    /// Error arising from the `gif` crate decoding process.
    Gif(gif::DecodingError),
    /// Error arising from the `gif-dispose` crate "rendering" process.
    GifDispose(String),
}

impl From<io::Error> for DecodeError {
    fn from(inner: io::Error) -> Self {
        DecodeError::Io(inner)
    }
}
impl From<gif::DecodingError> for DecodeError {
    fn from(inner: gif::DecodingError) -> Self {
        DecodeError::Gif(inner)
    }
}
impl From<Box<Error>> for DecodeError {
    fn from(inner: Box<Error>) -> Self {
        DecodeError::GifDispose(format!("{}", inner))
    }
}

impl Error for DecodeError {
    fn description(&self) -> &str { "GIF animation decode error" }
    fn cause(&self) -> Option<&Error> {
        match *self {
            DecodeError::Io(ref e) => Some(e),
            DecodeError::Gif(ref e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DecodeError::Io(ref e) => write!(fmt, "I/O error while decoding GIF: {}", e),
            DecodeError::Gif(ref e) => write!(fmt, "cannot decode GIF file: {}", e),
            DecodeError::GifDispose(ref e) => write!(fmt, "GIF rendering error: {}", e),
        }
    }
}


/// Decode animated GIF from given file.
pub fn decode_from_file<P: AsRef<Path>>(path: P) -> Result<GifAnimation, DecodeError> {
    let path = path.as_ref();
    trace!("Loading animated GIF from {}", path.display());
    let mut file = File::open(path)?;
    decode(&mut file)
}

/// Decode animated GIF from given reader.
pub fn decode<R: Read>(input: &mut R) -> Result<GifAnimation, DecodeError> {
    let mut decoder = gif::Decoder::new(input);
    decoder.set(gif::ColorOutput::Indexed);
    decoder.set(MEMORY_LIMIT);

    let mut reader = decoder.read_info()?;
    let width = reader.width();
    let height = reader.height();
    let palette = reader.global_palette()
        .map(|p| p.to_vec()).unwrap_or_else(Vec::new);
    let bg_color = reader.bg_color();

    // Read the frames and "draw" them on a virtual screen to ensure
    // that the frame disposal mechanics are applied correctly.
    let mut screen = Screen::new(&reader);
    let mut frames = vec![];
    while let Some(frame) = reader.read_next_frame()? {
        screen.blit(&frame)?;

        // Get the current pixels of the GIF logical screen as raw bytes
        // in order to make a new frame for the current state of the animation.
        let pixels_rgba: &[_] = &*screen.pixels;
        let pixel_bytes: &[u8] = unsafe {
            let ptr = pixels_rgba as *const _ as *const u8;
            slice::from_raw_parts(ptr, pixels_rgba.len() * RGBA_SIZE_BYTES)
        };

        // Adjust the metadata of the frame to use the correct size & position.
        let mut frame = GifFrame::from_rgba(
            /* metadata */ frame, screen.width, screen.height, pixel_bytes);
        frame.metadata.top = 0;
        frame.metadata.left = 0;
        frame.metadata.width = screen.width as u16;
        frame.metadata.height = screen.height as u16;
        frames.push(frame);
    }

    debug!("Animated GIF successfully loaded: {}x{} with {} frames",
        width, height, frames.len());
    Ok(GifAnimation{width, height, palette, bg_color, frames})
}

const RGBA_SIZE_BYTES: usize = 4;


// Encoding animated GIFs

/// Quality parameter for the NeuQuant color quantizer.
/// Range 1..=30. Lower values mean better quality.
const COLOR_SAMPLE_FACTION: i32 = 12;
// TODO: make this an Engine configuration parameter

/// Encode animated GIF.
pub fn encode<W: Write>(anim: &GifAnimation, output: W) -> io::Result<()> {
    let output = BgColorFixer::new(anim.bg_color.map(|i| i as u8), output);
    let mut encoder =
        gif::Encoder::new(output, anim.width, anim.height, &*anim.palette)?;
    encoder.set(gif::Repeat::Infinite)?;

    for (i, frame) in anim.iter_frames().enumerate() {
        trace!("Writing frame #{}", i + 1);
        let mut gif_frame = frame.metadata.clone();

        let (buffer, palette, transparent) = quantize_image(&frame.image);
        gif_frame.buffer = buffer.into();
        gif_frame.palette = Some(palette);
        gif_frame.transparent = transparent;

        encoder.write_frame(&gif_frame)?;
    }
    Ok(())
}

/// Encode animated GIF with its frames modified (replaced with given images).
/// Original animation will be used to provide metadata for GIF frames
/// (frame delays, transitions, etc.).
pub fn encode_modified<W: Write>(orig_anim: &GifAnimation,
                                 images: Vec<DynamicImage>,
                                 output: W) -> io::Result<()> {
    assert_eq!(orig_anim.frames_count(), images.len());

    // Create a new GifAnimation which is a shallow copy of the frame metadata,
    // where frame images are replaced with given DynamicImages.
    let mut new_frames = vec![];
    for (orig_frame, image) in orig_anim.iter_frames().zip(images.into_iter()) {
        let new_frame = GifFrame{
            image: image,
            metadata: orig_frame.metadata.clone(),
        };
        new_frames.push(new_frame);
    }
    let new_anim = GifAnimation{
        frames: new_frames,
        // Copy the rest of animation data
        // (can't use struct unpacking as it requires ownership of the source).
        width: orig_anim.width,
        height: orig_anim.height,
        palette: orig_anim.palette.clone(),
        bg_color: orig_anim.bg_color,
    };

    encode(&new_anim, output)
}

/// Low-level function that performs color quantization of an image.
///
/// Returns (buffer, palette, transparent) where:
/// * `buffer` is the image where pixels are palette indexes
/// * `palette` is a contiguous buffer of RGB colors in the palette used
/// * `transparent` is optional palette index of the transparent color
pub fn quantize_image(image: &DynamicImage) -> (Vec<u8>, Vec<u8>, Option<u8>) {
    let mut pixels = image.raw_pixels().to_owned();

    //
    // This is essentially gif::Frame::from_rgba().
    // The code is lifted here so that we can adjust the crucial COLOR_SAMPLE_FACTION
    // passed to the color quantizer.
    // (The original default of 1 makes it way too slow for most practical purposes).
    //

    let mut transparent = None;
    for pix in pixels.chunks_mut(4) {
        if pix[3] != 0 {
            pix[3] = 0xFF;
        } else {
            transparent = Some([pix[0], pix[1], pix[2], pix[3]])
        }
    }

    let quantizer = NeuQuant::new(COLOR_SAMPLE_FACTION, 256, &pixels[..]);

    let buffer = pixels.chunks(RGBA_SIZE_BYTES)
        .map(|pix| quantizer.index_of(pix) as u8)
        .collect();
    let palette = quantizer.color_map_rgb();
    let transparent = transparent.map(|t| quantizer.index_of(&t) as u8);

    (buffer, palette, transparent)
}

/// A really silly hack to work around the fact that `gif` crate doesn't allow to pass
/// the GIF's background color when encoding the image.
///
/// This structure is simply a writer that intercepts the first few bytes of the GIF,
/// replaces the 0 which `gif` puts as bg_color with the actual one,
/// and otherwise just passes the bytes through to the inner writer.
#[derive(Debug)]
struct BgColorFixer<W: Write> {
    bg_color: Option<u8>,
    buffer: Vec<u8>,
    writer: W,
}
impl<W: Write> BgColorFixer<W> {
    #[inline]
    pub fn new(bg_color: Option<u8>, writer: W) -> Self {
        BgColorFixer{
            bg_color: bg_color,
            buffer: Vec::with_capacity(BGCOLOR_OFFSET + 1),
            writer: writer,
        }
    }
}
impl<W: Write> BgColorFixer<W> {
    fn is_noop(&self) -> bool {
        self.bg_color.is_none() || self.buffer.len() > BGCOLOR_OFFSET
    }
}
impl<W: Write> Write for BgColorFixer<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.is_noop() {
            return self.writer.write(buf);
        }

        let count = self.buffer.write(buf)?;
        if self.buffer.len() > BGCOLOR_OFFSET {
            self.buffer[BGCOLOR_OFFSET] = self.bg_color.unwrap();
            self.writer.write(&self.buffer[..])?;
        }
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.is_noop() {
            return self.writer.flush();
        }
        Ok(())
    }
}
// http://giflib.sourceforge.net/whatsinagif/bits_and_bytes.html
const BGCOLOR_OFFSET: usize = 11;
