mod append;
mod cause;
mod valid;

pub use cause::*;
pub use valid::*;

/// Moral equivalent of TryFrom for validation purposes
pub trait ValidFrom<T>: Sized {
    type Error;
    type Trace;
    fn valid_from(a: T) -> Valid<Self, Self::Error, Self::Trace>;
}

/// Moral equivalent of TryInto for validation purposes
pub trait ValidInto<T> {
    type Error;
    type Trace;
    fn valid_into(self) -> Valid<T, Self::Error, Self::Trace>;
}

/// A blanket implementation for ValidateInto
impl<S, T: ValidFrom<S>> ValidInto<T> for S {
    type Error = T::Error;
    type Trace = T::Trace;
    fn valid_into(self) -> Valid<T, Self::Error, Self::Trace> {
        T::valid_from(self)
    }
}
