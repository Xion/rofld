//! Extension module for Rust itself.

#![allow(dead_code)]


/// Additional mutation methods for `Option`.
///
/// This is essentially the same thing that `#[feature(option_entry)]` solves.
pub trait OptionMutExt<T> {
    /// Replace an existing value with a new one.
    ///
    /// Returns the previous value if it was present, or `None` if no replacement was made.
    fn replace(&mut self, val: T) -> Option<T>;

    /// Replace existing value with result of given closure.
    ///
    /// Returns the previous value if it was present, or `None` if no replacement was made.
    fn replace_with<F: FnOnce() -> T>(&mut self, f: F) -> Option<T>;

    /// Set the "default" value of `Option` (if it didn't have one before)
    /// and return a mutable reference to the final value (old or new one).
    ///
    /// This is identical to unstable `Option::get_or_insert`.
    fn set_default(&mut self, val: T) -> &mut T;

    /// Set the "default" value of `Option` (if it didn't have one before)
    /// by evaluating given closure,
    /// and return a mutable reference to the final value (old or new one).
    ///
    /// This is identical to unstable `Option::get_or_insert_with`.
    fn set_default_with<F: FnOnce() -> T>(&mut self, f: F) -> &mut T;
}

impl<T> OptionMutExt<T> for Option<T> {
    fn replace(&mut self, val: T) -> Option<T> {
        self.replace_with(move || val)
    }

    fn replace_with<F: FnOnce() -> T>(&mut self, f: F) -> Option<T> {
        if self.is_some() {
            let result = self.take();
            *self = Some(f());
            result
        } else {
            None
        }
    }

    fn set_default(&mut self, val: T) -> &mut T {
        self.set_default_with(move || val)
    }

    fn set_default_with<F: FnOnce() -> T>(&mut self, f: F) -> &mut T {
        if self.is_none() {
            *self = Some(f());
        }
        self.as_mut().unwrap()
    }
}
