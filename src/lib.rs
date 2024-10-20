mod append;
mod cause;
mod valid;

pub use cause::*;
pub use valid::*;

/// Moral equivalent of TryFrom for validation purposes
pub trait ValidateFrom<T>: Sized {
    type Error;
    type Trace;
    fn validate_from(a: T) -> Valid<Self, Self::Error, Self::Trace>;
}

/// Moral equivalent of TryInto for validation purposes
pub trait ValidateInto<T> {
    type Error;
    type Trace;
    fn validate_into(self) -> Valid<T, Self::Error, Self::Trace>;
}

/// A blanket implementation for ValidateInto
impl<S, T: ValidateFrom<S>> ValidateInto<T> for S {
    type Error = T::Error;
    type Trace = T::Trace;
    fn validate_into(self) -> Valid<T, Self::Error, Self::Trace> {
        T::validate_from(self)
    }
}
