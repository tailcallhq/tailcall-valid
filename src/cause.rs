use std::{collections::VecDeque, fmt::Display};

use derive_setters::Setters;
use thiserror::Error;

#[derive(Clone, PartialEq, Debug, Setters, Error)]
pub struct Cause<E, T> {
    pub error: E,
    #[setters(skip)]
    pub trace: VecDeque<T>,
}

impl<E: Display, T: Display> Display for Cause<E, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, entry) in self.trace.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{entry}")?;
        }
        write!(f, "] {}", self.error)?;
        Ok(())
    }
}

impl<E, T> Cause<E, T> {
    pub fn new(e: E) -> Self {
        Cause {
            error: e,
            trace: Default::default(),
        }
    }

    pub fn trace(mut self, t: T) -> Self {
        self.trace.push_front(t);
        self
    }

    pub fn transform<E1>(self, e: impl Fn(E) -> E1) -> Cause<E1, T> {
        Cause {
            error: e(self.error),
            trace: self.trace,
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    #[test]
    fn test_display() {
        use super::Cause;
        let cause = Cause::new("error").trace("trace0").trace("trace1");
        assert_eq!(cause.to_string(), "[trace1, trace0] error");
    }
}
