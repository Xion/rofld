//! Utility code.

pub mod animated_gif;


/// Derive an implementation of From<InnerType> for one variant of a unary enum.
/// Adapter from the source of error_derive crate.
macro_rules! derive_enum_from(
    ($inner:ty => $enum_:ident::$variant:ident) => {
        impl ::std::convert::From<$inner> for $enum_ {
            fn from(v: $inner) -> $enum_ {
                $enum_::$variant(v)
            }
        }
    }
);

/// Converts a value to a "static" (though not &'static) str
/// so it can be used with APIs that only accept borrowed strings.
macro_rules! to_static_str(
    ($v:expr) => ({
        lazy_static! {
            static ref DUMMY: String = $v.to_string();
        }
        &*DUMMY as &str
    })
);
