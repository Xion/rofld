//! Extension module, gluing together & enhancing the third-party libraries.

pub mod hyper {
    use futures::{BoxFuture, future, Future, Stream};
    use hyper::{Body, Error};


    /// Trait with additional methods for the Hyper Body object.
    pub trait BodyExt {
        fn into_bytes(self) -> BoxFuture<Vec<u8>, Error>;
    }

    impl BodyExt for Body {
        fn into_bytes(self) -> BoxFuture<Vec<u8>, Error> {
            self.fold(vec![], |mut buf, chunk| {
                buf.extend_from_slice(&*chunk);
                future::ok::<_, Error>(buf)
            }).boxed()
        }
    }
}
