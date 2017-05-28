//! Module implementing image captioning.

mod error;
mod task;
mod text;


use std::sync::{Arc, Mutex, TryLockError};
use std::time::Duration;

use atomic::{Atomic, Ordering};
use futures::{BoxFuture, Future, future};
use futures_cpupool::{self, CpuPool};
use rand::{self, thread_rng};
use tokio_timer::Timer;

use args::Resource;
use model::ImageMacro;
use resources::{Cache, list_fonts, list_templates};
use self::error::CaptionError;
use self::task::{CaptionOutput, CaptionTask};


/// Renders image macros into captioned images.
pub struct Captioner {
    pool: Mutex<CpuPool>,
    cache: Arc<Cache>,
    timer: Timer,
    // Configuration params.
    task_timeout: Atomic<Duration>,
}

impl Captioner {
    #[inline]
    fn new() -> Self {
        let pool = Mutex::new(Self::pool_builder().create());
        let cache = Arc::new(Cache::new());
        let timer = Timer::default();

        let task_timeout = Atomic::new(Duration::from_secs(0));

        Captioner{pool, cache, timer, task_timeout}
    }

    #[inline]
    #[doc(hidden)]
    fn pool_builder() -> futures_cpupool::Builder {
        let mut builder = futures_cpupool::Builder::new();
        builder.name_prefix("caption-");
        builder.after_start(|| trace!("Worker thread created in Captioner::pool"));
        builder.before_stop(|| trace!("Stopping worker thread in Captioner::pool"));
        builder
    }
}

impl Captioner {
    #[inline]
    pub fn cache(&self) -> &Cache {
        &*self.cache
    }
}

// Configuration tweaks.
impl Captioner {
    #[inline]
    pub fn set_thread_count(&self, count: usize) -> &Self {
        trace!("Setting thread count for image captioning to {}", count);

        let mut builder = Self::pool_builder();
        if count > 0 {
            builder.pool_size(count);
        }

        let pool = builder.create();
        *self.pool.lock().unwrap() = pool;
        self
    }

    #[inline]
    pub fn set_task_timeout(&self, timeout: Duration) -> &Self {
        let secs = timeout.as_secs();
        if secs > 0 {
            trace!("Setting caption request timeout to {} secs", secs);
        } else {
            trace!("Disabling caption request timeout");
        }
        self.task_timeout.store(timeout, Ordering::Relaxed);
        self
    }

    /// Fill the cache for given type of resource.
    pub fn preload(&self, what: Resource) {
        let mut rng = thread_rng();
        match what {
            Resource::Template => {
                let capacity = self.cache.templates().capacity();
                debug!("Preloading up to {} templates", capacity);
                // TODO: the sampling here is O(N_t*C), so it can be quadratic;
                // pick a better method (probably the random_choice crate)
                for template in rand::sample(&mut rng, list_templates(), capacity) {
                    self.cache.load_template(&template);
                }
            }
            Resource::Font => {
                let capacity = self.cache.fonts().capacity();
                debug!("Preloading up to {} fonts", capacity);
                for font in rand::sample(&mut rng, list_fonts(), capacity) {
                    self.cache.load_font(&font);
                }
            }
        }
    }
}

// Rendering code.
impl Captioner {
    /// Render an image macro as PNG.
    /// The rendering is done in a separate thread.
    pub fn render(&self, im: ImageMacro) -> BoxFuture<CaptionOutput, CaptionError> {
        let pool = match self.pool.try_lock() {
            Ok(p) => p,
            Err(TryLockError::WouldBlock) => {
                // This should be only possible when set_thread_count() happens
                // to have been called at the exact same moment.
                warn!("Could not immediately lock CpuPool to render {:?}", im);
                // TODO: retry a few times, probably with exponential backoff
                return future::err(CaptionError::Unavailable).boxed();
            },
            Err(e) => {
                // TODO: is this a fatal error?
                error!("Error while locking CpuPool for rendering {:?}: {}", im, e);
                return future::err(CaptionError::Unavailable).boxed();
            },
        };

        // Spawn a new task in the thread pool for the rendering process.
        let task_future = pool.spawn_fn({
            let im_repr = format!("{:?}", im);
            let task = CaptionTask{
                image_macro: im,
                cache: self.cache.clone(),
            };
            move || {
                match task.perform() {
                    Ok(out) => {
                        debug!("Successfully rendered {} as {:?}, final result size: {} bytes",
                            im_repr, out.format, out.bytes.len());
                        future::ok(out)
                    },
                    Err(e) => {
                        error!("Failed to render image macro {}: {}", im_repr, e);
                        future::err(e)
                    },
                }
            }
        });

        // Impose a timeout on the task.
        let max_duration = self.task_timeout.load(Ordering::Relaxed);
        if max_duration.as_secs() > 0 {
            // TODO: this doesn't seem to actually kill the underlying thread,
            // figure out how to do that
            self.timer.timeout(task_future, max_duration).boxed()
        } else {
            task_future.boxed()
        }
    }
}

lazy_static! {
    /// The singleton instance of Captioner.
    pub static ref CAPTIONER: Arc<Captioner> = Arc::new(Captioner::new());
}
