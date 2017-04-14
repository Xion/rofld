//! Extension module, gluing together & enhancing the third-party libraries.

pub mod hyper {
    use futures::{future, Future, Stream};
    use hyper::{Body, Error};

    use super::futures::{ArcFuture, FutureExt};


    /// Trait with additional methods for the Hyper Body object.
    pub trait BodyExt {
        fn into_bytes(self) -> ArcFuture<Vec<u8>, Error>;
    }

    impl BodyExt for Body {
        fn into_bytes(self) -> ArcFuture<Vec<u8>, Error> {
            self.fold(vec![], |mut buf, chunk| {
                buf.extend_from_slice(&*chunk);
                future::ok::<_, Error>(buf)
            }).arc()
        }
    }
}


pub mod futures {
    use std::sync::{Arc, Mutex};

    use futures::{Future, Poll};


    /// Trait with additional methods for Futures.
    pub trait FutureExt : Future + 'static {
        fn arc(self) -> ArcFuture<Self::Item, Self::Error>;
    }

    impl<F: Future + 'static> FutureExt for F {
        fn arc(self) -> ArcFuture<Self::Item, Self::Error> {
            ArcFuture(Arc::new(Mutex::new(self)))
        }
    }


    /// A type alias for Arc<Future>.
    pub struct ArcFuture<T, E>(Arc<Mutex<Future<Item=T, Error=E>>>);

    impl<T, E> Future for ArcFuture<T, E> {
        type Item = T;
        type Error = E;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            self.0.try_lock().expect("ArcFuture mutex poisoned").poll()
        }
    }
}
