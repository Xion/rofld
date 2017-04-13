//!
//! rofld  -- Lulz on demand
//!

             extern crate ansi_term;
             extern crate futures;
             extern crate glob;
             extern crate hyper;
             extern crate image;
             extern crate isatty;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate maplit;
             extern crate rusttype;
             extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
             extern crate serde_qs;
             extern crate slog_envlogger;
             extern crate slog_stdlog;
             extern crate slog_stream;
             extern crate time;
#[macro_use] extern crate try_opt;

// `slog` must precede `log` in declarations here, because we want to simultaneously:
// * use the standard `log` macros (at least for a while)
// * be able to initialize the slog logger using slog macros like o!()
#[macro_use] extern crate slog;
#[macro_use] extern crate log;


mod ext;
mod logging;
mod templates;


use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use futures::Future;
use futures::future::{self, BoxFuture};
use hyper::{Get, Post, StatusCode};
use hyper::server::{Http, Service, Request, Response};
use image::{GenericImage, Rgb};
use rusttype::{FontCollection, Point, Scale};

use ext::hyper::BodyExt;


const HOST: &'static str = "0.0.0.0";
const PORT: u16 = 1337;

lazy_static! {
    static ref FONT_DIR: PathBuf =
        env::current_dir().unwrap().join("data").join("fonts");
}


fn main() {
    // TODO: logging verbosity command line flag
    logging::init(1).unwrap();

    let addr = format!("{}:{}", HOST, PORT).parse().unwrap();
    info!("Starting server to listen on {}...", addr);
    let server = Http::new().bind(&addr, || Ok(Rofl)).unwrap();

    debug!("Entering event loop...");
    server.run().unwrap();
}


/// Hyper async service implementing ALL the functionality.
pub struct Rofl;

impl Service for Rofl {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = BoxFuture<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        self.log(&req);

        match (req.method(), req.path()) {
            (_, "/caption") => return self.handle_caption(req),
            (&Get, "/templates") => return self.handle_list_templates(req),
            _ => {},
        }

        let error_resp = match (req.method(), req.path()) {
            (&Get, "/") => Response::new().with_status(StatusCode::MethodNotAllowed),
            _ => Response::new().with_status(StatusCode::NotFound),
        };
        future::ok(error_resp).boxed()
    }
}

// Request handlers.
impl Rofl {
    /// Handle the image captioning request.
    fn handle_caption(&self, request: Request) ->  <Self as Service>::Future {
        let (method, url, _, _, body) = request.deconstruct();
        body.into_bytes().map(move |bytes| {
            let parsed_im: Result<_, Box<Error>> = match method {
                Get => {
                    trace!("Decoding image macro spec from {} bytes of query string",
                        url.query().map(|q| q.len()).unwrap_or(0));
                    serde_qs::from_str(url.query().unwrap_or("")).map_err(Into::into)
                },
                Post => {
                    trace!("Decoding image macro spec from {} bytes of JSON", bytes.len());
                    serde_json::from_reader(&*bytes).map_err(Into::into)
                },
                _ => return Response::new().with_status(StatusCode::MethodNotAllowed),
            };

            let im: ImageMacro = match parsed_im {
                Ok(im) => im,
                Err(e) => {
                    error!("Failed to decode image macro: {}", e);
                    return error_response(
                        StatusCode::BadRequest, "cannot decode request");
                },
            };
            debug!("Decoded {:?}", im);

            let mut image = vec![];
            match im.render(&mut image) {
                Ok(_) => Response::new().with_body(image),
                Err(e) => {
                    error!("Failed to render image macro {:?}: {}", im, e);
                    e.into()
                },
            }
        }).boxed()
    }

    /// Handle the template listing request.
    fn handle_list_templates(&self, _: Request) -> <Self as Service>::Future {
        let template_names = templates::list();
        let response = Response::new()
            .with_body(json!(template_names).to_string());
        future::ok(response).boxed()
    }
}

impl Rofl {
    #[inline]
    fn log(&self, req: &Request) {
        info!("{} {} {}{} {}",
            req.remote_addr().map(|a| format!("{}", a.ip())).unwrap_or_else(|| "-".to_owned()),
            format!("{}", req.method()).to_uppercase(),
            req.path(),
            req.query().map(|q| format!("?{}", q)).unwrap_or_else(String::new),
            req.version());
    }
}


/// Describes an image macro, used as an input structure.
#[derive(Deserialize)]
struct ImageMacro {
    template: String,
    width: Option<u32>,
    height: Option<u32>,

    font: Option<String>,
    top_text: Option<String>,
    middle_text: Option<String>,
    bottom_text: Option<String>,
}
impl ImageMacro {
    /// Render the image macro as PNG into the specified Writer.
    pub fn render<W: Write>(&self, writer: &mut W) -> Result<(), CaptionError> {
        debug!("Rendering {:?}", self);

        let img = templates::load(&self.template)
            .ok_or_else(|| CaptionError::Template(self.template.clone()))?;

        // Resize the image to fit within the given dimensions.
        // Note that the resizing preserves original aspect, so the final image
        // may be smaller than requested.
        let (orig_width, orig_height) = img.dimensions();
        let target_width = self.width.unwrap_or(orig_width);
        let target_height = self.height.unwrap_or(orig_height);
        debug!("Resizing template `{}` from {}x{} to {}x{}",
            self.template, orig_width, orig_height, target_width, target_height);
        let mut img = img.resize(target_width, target_height, image::FilterType::Lanczos3);

        // TODO: other texts and better
        if let Some(ref bottom_text) = self.bottom_text {
            let font_name = self.font.as_ref().map(|s| s.as_str()).unwrap_or("Impact");
            let font_file = fs::File::open(FONT_DIR.join(format!("{}.ttf", font_name)))
                .map_err(CaptionError::Font)?;
            let font_bytes: Vec<_> = font_file.bytes().map(Result::unwrap).collect();
            let font = FontCollection::from_bytes(&*font_bytes).into_font().unwrap();
            let size = 64.0;
            let pos = Point{x: 0.0, y: target_height as f32 - size};
            font.layout(bottom_text, Scale::uniform(size), pos)
                .scan(0.0, |caret, g| {
                    g.draw(|x, y, v| {
                        let x = (pos.x + *caret) as u32 + x;
                        let y = pos.y as u32 + y;
                        let v = (v * 255f32) as u8;
                        img.as_mut_rgb8().unwrap().put_pixel(x, y, Rgb{data: [v, v, v]});
                    });
                    // TODO: probably need to use advance_width + font.pair_kerning instead
                    let rect = g.pixel_bounding_box().unwrap();
                    *caret += rect.width() as f32;
                    Some(())
                })
                .count();  // just to consume the iterator
        }

        let (width, height) = img.dimensions();
        image::png::PNGEncoder::new(writer)
            .encode(&*img.raw_pixels(), width, height, image::ColorType::RGB(8))
            .map_err(CaptionError::Encode)
    }
}
impl fmt::Debug for ImageMacro {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut ds = fmt.debug_struct("ImageMacro");
        ds.field("template", &self.template);

        if let Some(ref width) = self.width {
            ds.field("width", width);
        }
        if let Some(ref height) = self.height {
            ds.field("height", height);
        }

        if let Some(ref text) = self.top_text {
            ds.field("top_text", text);
        }
        if let Some(ref text) = self.middle_text {
            ds.field("middle_text", text);
        }
        if let Some(ref text) = self.bottom_text {
            ds.field("bottom_text", text);
        }

        ds.finish()
    }
}

/// Error that may occur during the captioning.
#[derive(Debug)]
enum CaptionError {
    Template(String),
    Font(io::Error),
    Encode(io::Error),
}
impl CaptionError {
    #[inline]
    pub fn status_code(&self) -> StatusCode {
        match *self {
            CaptionError::Template(..) => StatusCode::NotFound,
            CaptionError::Font(..) => StatusCode::NotFound,
            CaptionError::Encode(..) => StatusCode::InternalServerError,
        }
    }
}
impl Error for CaptionError {
    fn description(&self) -> &str { "captioning error" }
    fn cause(&self) -> Option<&Error> {
        match *self {
            CaptionError::Encode(ref e) => Some(e),
            _ => None,
        }
    }
}
impl fmt::Display for CaptionError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CaptionError::Template(ref t) => write!(fmt, "cannot find template `{}`", t),
            CaptionError::Font(ref t) => write!(fmt, "cannot find font `{}`", t),
            CaptionError::Encode(ref e) => write!(fmt, "failed to encode the  final image: {}", e),
        }
    }
}
impl Into<Response> for CaptionError {
    fn into(self) -> Response {
        error_response(self.status_code(), format!("{}", self))
    }
}


fn error_response<T: ToString>(status_code: StatusCode, message: T) -> Response {
    let message = message.to_string();
    Response::new()
        .with_status(status_code)
        .with_body(json!({"error": message}).to_string())
}
