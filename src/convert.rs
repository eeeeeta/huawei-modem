//! Reimplementations of unstable stdlib conversions.

/// A type that can't be created - essentially like the unstable `!`
#[derive(Debug, Copy, Clone)]
pub enum Infallible { }

/// Copy of the unstable `std::convert::TryFrom`.
pub trait TryFrom<T>: Sized {
    type Error;
    fn try_from(value: T) -> Result<Self, Self::Error>;
}
