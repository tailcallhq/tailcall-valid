use super::append::Append;
use super::Cause;

/// A validation type that can represent either a successful value of type `A`
/// or a collection of validation errors of type `E` with trace context `T`.
///
/// `Valid` is useful for accumulating multiple validation errors rather than
/// stopping at the first error encountered.
#[derive(Debug, PartialEq)]
pub struct Valid<A, E, T>(Result<A, Vec<Cause<E, T>>>);

/// Trait for types that can perform validation operations.
///
/// This trait provides a rich set of combinators for working with validations,
/// allowing you to chain, combine and transform validation results.
pub trait Validator<A, E, T>: Sized {
    /// Maps a function over the successful value, transforming it to a new type.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let valid = Valid::<i32, (), ()>::succeed(1);
    /// let result = valid.map(|x| x.to_string());
    /// assert_eq!(result, Valid::succeed("1".to_string()));
    /// ```
    fn map<A1>(self, f: impl FnOnce(A) -> A1) -> Valid<A1, E, T> {
        Valid(self.to_result().map(f))
    }

    /// Executes a side effect function if the validation is successful.
    /// The original value is preserved.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let mut sum = 0;
    /// let valid = Valid::<i32, (), ()>::succeed(5);
    /// valid.foreach(|x| sum += x);
    /// assert_eq!(sum, 5);
    /// ```
    fn foreach(self, mut f: impl FnMut(A)) -> Valid<A, E, T>
    where
        A: Clone,
    {
        match self.to_result() {
            Ok(a) => {
                f(a.clone());
                Valid::succeed(a)
            }
            Err(e) => Valid(Err(e)),
        }
    }

    /// Returns true if the validation is successful.
    fn is_succeed(&self) -> bool;

    /// Returns true if the validation contains errors.
    fn is_fail(&self) -> bool;

    /// Combines two validations, keeping the result of the second one if both succeed.
    /// If either validation fails, all errors are collected.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let v1 = Valid::<i32, &str, ()>::succeed(1);
    /// let v2 = Valid::<&str, &str, ()>::succeed("ok");
    /// assert_eq!(v1.and(v2), Valid::succeed("ok"));
    /// ```
    fn and<A1>(self, other: Valid<A1, E, T>) -> Valid<A1, E, T> {
        self.zip(other).map(|(_, a1)| a1)
    }

    /// Combines two validations into a tuple of their results.
    /// If either validation fails, all errors are collected.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let v1 = Valid::<i32, &str, ()>::succeed(1);
    /// let v2 = Valid::<&str, &str, ()>::succeed("ok");
    /// assert_eq!(v1.zip(v2), Valid::succeed((1, "ok")));
    /// ```
    fn zip<A1>(self, other: Valid<A1, E, T>) -> Valid<(A, A1), E, T> {
        match self.to_result() {
            Ok(a) => match other.0 {
                Ok(a1) => Valid(Ok((a, a1))),
                Err(e1) => Valid(Err(e1)),
            },
            Err(mut e1) => match other.0 {
                Ok(_) => Valid(Err(e1)),
                Err(e2) => {
                    e1.extend(e2);
                    Valid(Err(e1))
                }
            },
        }
    }

    /// Starts a fusion chain of validations. This allows combining multiple
    /// validation results using the `Append` trait.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let v1: Valid<Vec<i32>, (), ()> = Valid::succeed(vec![1, 2]);
    /// let v2: Valid<Vec<i32>, (), ()> = Valid::succeed(vec![3, 4]);
    /// let result = v1.fuse(v2);
    /// assert_eq!(result.to_result().unwrap(), (vec![1, 2], vec![3, 4]));
    /// ```
    fn fuse<A1>(self, other: Valid<A1, E, T>) -> Fusion<(A, A1), E, T> {
        Fusion(self.zip(other))
    }

    /// Adds trace context to any errors in the validation.
    /// Successful validations are unaffected.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let result = Valid::<(), &str, &str>::fail("error")
    ///     .trace("field_name")
    ///     .trace("form");
    /// ```
    fn trace(self, trace: impl Into<T> + Clone) -> Valid<A, E, T> {
        let valid = self.to_result();
        if let Err(error) = valid {
            return Valid(Err(error
                .into_iter()
                .map(|cause| cause.trace(trace.clone().into()))
                .collect()));
        }

        Valid(valid)
    }

    /// Handles both success and failure cases of a validation.
    ///
    /// - If successful, applies the `ok` function to the value
    /// - If failed, calls the `err` function and combines any new errors
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let valid = Valid::<i32, &str, ()>::succeed(1);
    /// let result = valid.fold(
    ///     |n| Valid::succeed(n + 1),
    ///     || Valid::succeed(0)
    /// );
    /// assert_eq!(result, Valid::succeed(2));
    /// ```
    fn fold<A1>(
        self,
        ok: impl FnOnce(A) -> Valid<A1, E, T>,
        err: impl FnOnce() -> Valid<A1, E, T>,
    ) -> Valid<A1, E, T> {
        match self.to_result() {
            Ok(a) => ok(a),
            Err(e) => Valid::<A1, E, T>(Err(e)).and(err()),
        }
    }

    /// Converts the validation into a Result.
    fn to_result(self) -> Result<A, Vec<Cause<E, T>>>;

    /// Chains a validation operation by applying a function to a successful value.
    /// If the original validation failed, the errors are propagated.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let valid = Valid::<i32, &str, ()>::succeed(1);
    /// let result = valid.and_then(|n| {
    ///     if n > 0 {
    ///         Valid::succeed(n * 2)
    ///     } else {
    ///         Valid::fail("must be positive")
    ///     }
    /// });
    /// assert_eq!(result, Valid::succeed(2));
    /// ```
    fn and_then<B>(self, f: impl FnOnce(A) -> Valid<B, E, T>) -> Valid<B, E, T> {
        match self.to_result() {
            Ok(a) => f(a),
            Err(e) => Valid(Err(e)),
        }
    }

    /// Converts a successful validation to `()`.
    /// Failed validations retain their errors.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let valid = Valid::<i32, &str, ()>::succeed(1);
    /// assert_eq!(valid.unit(), Valid::succeed(()));
    /// ```
    fn unit(self) -> Valid<(), E, T> {
        self.map(|_| ())
    }

    /// Wraps a successful value in Some(_).
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let valid = Valid::<i32, &str, ()>::succeed(1);
    /// assert_eq!(valid.some(), Valid::succeed(Some(1)));
    /// ```
    fn some(self) -> Valid<Option<A>, E, T> {
        self.map(Some)
    }

    /// Maps a successful validation to a constant value.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let valid = Valid::<i32, &str, ()>::succeed(1);
    /// assert_eq!(valid.map_to("ok"), Valid::succeed("ok"));
    /// ```
    fn map_to<B>(self, b: B) -> Valid<B, E, T> {
        self.map(|_| b)
    }

    /// Conditionally validates based on a predicate.
    /// If the predicate returns false, succeeds with ().
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let valid = Valid::<(), &str, ()>::fail("error");
    /// let result = valid.when(|| false);
    /// assert_eq!(result, Valid::succeed(()));
    /// ```
    fn when(self, f: impl FnOnce() -> bool) -> Valid<(), E, T> {
        if f() {
            self.unit()
        } else {
            Valid::succeed(())
        }
    }
}

impl<A, E, T> Valid<A, E, T> {
    /// Creates a new failed validation with a single error.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let result: Valid<(), i32, ()> = Valid::fail(1);
    /// assert!(result.is_fail());
    /// ```
    pub fn fail(e: E) -> Valid<A, E, T> {
        Valid(Err(vec![Cause {
            error: e,
            trace: Default::default(),
        }]))
    }

    /// Creates a new failed validation with an error and trace context.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let result = Valid::<(), &str, &str>::fail_at("error", "context");
    /// assert!(result.is_fail());
    /// ```
    pub fn fail_at(error: E, trace: T) -> Valid<A, E, T>
    where
        E: std::fmt::Debug,
    {
        let cause = Cause::new(error).trace(trace);
        Valid(Err(vec![cause]))
    }

    /// Creates a new successful validation containing the given value.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let result = Valid::<i32, (), ()>::succeed(42);
    /// assert!(result.is_succeed());
    /// ```
    pub fn succeed(a: A) -> Valid<A, E, T> {
        Valid(Ok(a))
    }

    /// Validates each item in an iterator using the provided validation function,
    /// collecting all errors that occur.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let numbers = vec![1, 2, 3];
    /// let result = Valid::from_iter(numbers, |n| {
    ///     if n % 2 == 0 {
    ///         Valid::<i32, String, ()>::succeed(n * 2)
    ///     } else {
    ///         Valid::<i32, String, ()>::fail(format!("{} is odd", n))
    ///     }
    /// });
    /// ```
    pub fn from_iter<B>(
        iter: impl IntoIterator<Item = A>,
        mut f: impl FnMut(A) -> Valid<B, E, T>,
    ) -> Valid<Vec<B>, E, T> {
        let mut values: Vec<B> = Vec::new();
        let mut errors: Vec<Cause<E, T>> = Vec::new();
        for a in iter.into_iter() {
            match f(a).to_result() {
                Ok(b) => values.push(b),
                Err(err) => errors.extend(err),
            }
        }

        if errors.is_empty() {
            Valid::succeed(values)
        } else {
            Valid::from(errors)
        }
    }

    /// Creates a new `Valid` from an `Option` value.
    /// If the option is `None`, creates a failed validation with the provided error.
    /// If the option is `Some`, creates a successful validation with the contained value.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let some_value = Some(42);
    /// let result: Valid<i32, &str, ()> = Valid::from_option(some_value, "error");
    /// assert_eq!(result, Valid::succeed(42));
    ///
    /// let none_value: Option<i32> = None;
    /// let result: Valid<i32, &str, ()> = Valid::from_option(none_value, "error");
    /// assert!(result.is_fail());
    /// ```
    pub fn from_option(option: Option<A>, e: E) -> Valid<A, E, T> {
        match option {
            Some(a) => Valid::succeed(a),
            None => Valid::fail(e),
        }
    }

    /// Creates a successful validation containing `None`.
    ///
    /// This is useful when you want to explicitly represent the absence of a value
    /// as a successful validation rather than an error condition.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::Valid;
    /// let result: Valid<Option<i32>, &str, ()> = Valid::none();
    /// assert_eq!(result, Valid::succeed(None));
    /// ```
    pub fn none() -> Valid<Option<A>, E, T> {
        Valid::succeed(None)
    }
}

impl<A, E, T> From<Cause<E, T>> for Valid<A, E, T> {
    /// Creates a failed validation from a single `Cause`.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator, Cause};
    /// let cause = Cause::new("error");
    /// let result: Valid<(), &str, ()> = Valid::from(cause);
    /// assert!(result.is_fail());
    /// ```
    fn from(value: Cause<E, T>) -> Self {
        Valid(Err(vec![value]))
    }
}

impl<A, E, T> From<Vec<Cause<E, T>>> for Valid<A, E, T> {
    /// Creates a failed validation from a vector of `Cause`s.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator, Cause};
    /// let causes = vec![Cause::new("error1"), Cause::new("error2")];
    /// let result: Valid<(), &str, ()> = Valid::from(causes);
    /// assert!(result.is_fail());
    /// ```
    fn from(value: Vec<Cause<E, T>>) -> Self {
        Valid(Err(value))
    }
}

impl<A, E, T> Validator<A, E, T> for Valid<A, E, T> {
    fn to_result(self) -> Result<A, Vec<Cause<E, T>>> {
        self.0
    }

    fn is_succeed(&self) -> bool {
        self.0.is_ok()
    }

    fn is_fail(&self) -> bool {
        self.0.is_err()
    }
}

/// A type that allows chaining multiple validations together while combining their results.
///
/// `Fusion` is particularly useful when you want to accumulate values from multiple
/// successful validations into a single composite value.
pub struct Fusion<A, E, T>(Valid<A, E, T>);
impl<A, E, T> Fusion<A, E, T> {
    /// Combines this fusion with another validation, using the `Append` trait to
    /// combine their successful values.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let v1: Valid<Vec<i32>, (), ()> = Valid::succeed(vec![1, 2]);
    /// let v2: Valid<Vec<i32>, (), ()> = Valid::succeed(vec![3, 4]);
    /// let fusion = v1.fuse(v2);
    /// let result = fusion.to_result().unwrap();
    /// assert_eq!(result, (vec![1, 2], vec![3, 4]));
    /// ```
    pub fn fuse<A1>(self, other: Valid<A1, E, T>) -> Fusion<A::Out, E, T>
    where
        A: Append<A1>,
    {
        Fusion(self.0.zip(other).map(|(a, a1)| a.append(a1)))
    }
}

impl<A, E, T> Validator<A, E, T> for Fusion<A, E, T> {
    fn to_result(self) -> Result<A, Vec<Cause<E, T>>> {
        self.0.to_result()
    }
    fn is_succeed(&self) -> bool {
        self.0.is_succeed()
    }
    fn is_fail(&self) -> bool {
        self.0.is_fail()
    }
}

impl<A, E, T> From<Result<A, Cause<E, T>>> for Valid<A, E, T> {
    /// Creates a `Valid` from a `Result` containing a single `Cause` as its error type.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator, Cause};
    /// let ok_result: Result<i32, Cause<&str, ()>> = Ok(42);
    /// let valid = Valid::from(ok_result);
    /// assert_eq!(valid, Valid::succeed(42));
    ///
    /// let err_result: Result<i32, Cause<&str, ()>> = Err(Cause::new("error"));
    /// let valid = Valid::from(err_result);
    /// assert!(valid.is_fail());
    /// ```
    fn from(value: Result<A, Cause<E, T>>) -> Self {
        match value {
            Ok(a) => Valid::succeed(a),
            Err(e) => Valid(Err(vec![e])),
        }
    }
}

impl<A, E, T> From<Result<A, Vec<Cause<E, T>>>> for Valid<A, E, T> {
    /// Creates a `Valid` from a `Result` containing multiple `Cause`s as its error type.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator, Cause};
    /// let ok_result: Result<i32, Vec<Cause<&str, ()>>> = Ok(42);
    /// let valid = Valid::from(ok_result);
    /// assert_eq!(valid, Valid::succeed(42));
    ///
    /// let err_result: Result<i32, Vec<Cause<&str, ()>>> = Err(vec![
    ///     Cause::new("error1"),
    ///     Cause::new("error2")
    /// ]);
    /// let valid = Valid::from(err_result);
    /// assert!(valid.is_fail());
    /// ```
    fn from(value: Result<A, Vec<Cause<E, T>>>) -> Self {
        match value {
            Ok(a) => Valid::succeed(a),
            Err(e) => Valid(Err(e)),
        }
    }
}

impl<A, E, T> From<Fusion<A, E, T>> for Valid<A, E, T> {
    /// Converts a `Fusion` back into a `Valid`.
    ///
    /// This is typically used at the end of a chain of `fuse` operations
    /// to convert the final result back into a `Valid`.
    ///
    /// # Examples
    /// ```
    /// use tailcall_valid::{Valid, Validator};
    /// let v1: Valid<Vec<i32>, (), ()> = Valid::succeed(vec![1]);
    /// let v2: Valid<Vec<i32>, (), ()> = Valid::succeed(vec![2]);
    /// let fusion = v1.fuse(v2);
    /// let result: Valid<(Vec<i32>, Vec<i32>), (), ()> = Valid::from(fusion);
    /// assert!(result.is_succeed());
    /// ```
    fn from(value: Fusion<A, E, T>) -> Self {
        Valid(value.to_result())
    }
}

impl<A, E, T> Clone for Valid<A, E, T>
where
    A: Clone,
    E: Clone,
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{Cause, Valid, Validator};

    #[test]
    fn test_ok() {
        let result = Valid::<i32, (), ()>::succeed(1);
        assert_eq!(result, Valid::succeed(1));
    }

    #[test]
    fn test_fail() {
        let result = Valid::<(), i32, ()>::fail(1);
        assert_eq!(result, Valid::fail(1));
    }

    #[test]
    fn test_validate_or_both_ok() {
        let result1 = Valid::<bool, i32, ()>::succeed(true);
        let result2 = Valid::<u8, i32, ()>::succeed(3);

        assert_eq!(result1.and(result2), Valid::succeed(3u8));
    }

    #[test]
    fn test_validate_or_first_fail() {
        let result1 = Valid::<bool, i32, ()>::fail(-1);
        let result2 = Valid::<u8, i32, ()>::succeed(3);

        assert_eq!(result1.and(result2), Valid::fail(-1));
    }

    #[test]
    fn test_validate_or_second_fail() {
        let result1 = Valid::<bool, i32, ()>::succeed(true);
        let result2 = Valid::<u8, i32, ()>::fail(-2);

        assert_eq!(result1.and(result2), Valid::fail(-2));
    }

    #[test]
    fn test_validate_all() {
        let input: Vec<i32> = [1, 2, 3].to_vec();
        let result: Valid<Vec<i32>, i32, ()> = Valid::from_iter(input, |a| Valid::fail(a * 2));
        assert_eq!(
            result,
            Valid::from(vec![Cause::new(2), Cause::new(4), Cause::new(6)])
        );
    }

    #[test]
    fn test_validate_all_ques() {
        let input: Vec<i32> = [1, 2, 3].to_vec();
        let result: Valid<Vec<i32>, i32, ()> = Valid::from_iter(input, |a| Valid::fail(a * 2));
        assert_eq!(
            result,
            Valid::from(vec![Cause::new(2), Cause::new(4), Cause::new(6)])
        );
    }

    #[test]
    fn test_ok_ok_cause() {
        let option: Option<i32> = None;
        let result: Valid<i32, i32, ()> = Valid::from_option(option, 1);
        assert_eq!(result, Valid::from(vec![Cause::new(1)]));
    }

    #[test]
    fn test_trace() {
        let result = Valid::<(), i32, String>::fail(1)
            .trace("A")
            .trace("B")
            .trace("C");

        let expected = Valid::from(vec![Cause {
            error: 1,
            trace: vec!["C".to_string(), "B".to_string(), "A".to_string()].into(),
        }]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_validate_fold_err() {
        let valid = Valid::<(), i32, ()>::fail(1);
        let result = valid.fold(
            |_| Valid::<(), i32, ()>::fail(2),
            || Valid::<(), i32, ()>::fail(3),
        );
        assert_eq!(result, Valid::from(vec![Cause::new(1), Cause::new(3)]));
    }

    #[test]
    fn test_validate_fold_ok() {
        let valid = Valid::<i32, i32, i32>::succeed(1);
        let result = valid.fold(Valid::<i32, i32, i32>::fail, || {
            Valid::<i32, i32, i32>::fail(2)
        });
        assert_eq!(result, Valid::fail(1));
    }

    #[test]
    fn test_to_result() {
        let result = Valid::<(), i32, i32>::fail(1).to_result().unwrap_err();
        assert_eq!(result, vec![Cause::new(1)]);
    }

    #[test]
    fn test_validate_both_ok() {
        let result1 = Valid::<bool, i32, i32>::succeed(true);
        let result2 = Valid::<u8, i32, i32>::succeed(3);

        assert_eq!(result1.zip(result2), Valid::succeed((true, 3u8)));
    }
    #[test]
    fn test_validate_both_first_fail() {
        let result1 = Valid::<bool, i32, i32>::fail(-1);
        let result2 = Valid::<u8, i32, i32>::succeed(3);

        assert_eq!(result1.zip(result2), Valid::fail(-1));
    }
    #[test]
    fn test_validate_both_second_fail() {
        let result1 = Valid::<bool, i32, i32>::succeed(true);
        let result2 = Valid::<u8, i32, i32>::fail(-2);

        assert_eq!(result1.zip(result2), Valid::fail(-2));
    }

    #[test]
    fn test_validate_both_both_fail() {
        let result1 = Valid::<bool, i32, i32>::fail(-1);
        let result2 = Valid::<u8, i32, i32>::fail(-2);

        assert_eq!(
            result1.zip(result2),
            Valid::from(vec![Cause::new(-1), Cause::new(-2)])
        );
    }

    #[test]
    fn test_and_then_success() {
        let result = Valid::<i32, i32, i32>::succeed(1).and_then(|a| Valid::succeed(a + 1));
        assert_eq!(result, Valid::succeed(2));
    }

    #[test]
    fn test_and_then_fail() {
        let result =
            Valid::<i32, i32, i32>::succeed(1).and_then(|a| Valid::<i32, i32, i32>::fail(a + 1));
        assert_eq!(result, Valid::fail(2));
    }

    #[test]
    fn test_foreach_succeed() {
        let mut a = 0;
        let result = Valid::<i32, i32, i32>::succeed(1).foreach(|v| a = v);
        assert_eq!(result, Valid::succeed(1));
        assert_eq!(a, 1);
    }

    #[test]
    fn test_foreach_fail() {
        let mut a = 0;
        let result = Valid::<i32, i32, i32>::fail(1).foreach(|v| a = v);
        assert_eq!(result, Valid::fail(1));
        assert_eq!(a, 0);
    }

    #[test]
    fn test_trace_owned_referenced() {
        let trace_value = "inner".to_string();

        let valid: Valid<((), ()), &str, String> = Valid::fail("fail")
            .trace(&trace_value)
            .zip(Valid::fail("fail 2").trace(trace_value))
            .trace("outer");

        let causes = valid.to_result().unwrap_err();

        assert_eq!(causes.len(), 2);
        assert_eq!(causes[0].to_string(), "[outer, inner] fail");
        assert_eq!(causes[1].to_string(), "[outer, inner] fail 2");
    }
    #[test]
    fn test_from_result_vec_causes_ok() {
        let ok_result: Result<i32, Vec<Cause<&str, ()>>> = Ok(42);
        let valid = Valid::from(ok_result);
        assert_eq!(valid, Valid::succeed(42));
    }

    #[test]
    fn test_from_result_vec_causes_err() {
        let err_result: Result<i32, Vec<Cause<&str, ()>>> = Err(vec![
            Cause::new("error1"),
            Cause::new("error2"),
        ]);
        let valid = Valid::from(err_result);
        let expected = Valid::from(vec![Cause::new("error1"), Cause::new("error2")]);
        assert_eq!(valid, expected);
        assert!(valid.is_fail());
    }

    #[test]
    fn test_from_result_vec_causes_empty_err() {
        let err_result: Result<i32, Vec<Cause<&str, ()>>> = Err(vec![]);
        let valid = Valid::from(err_result);
        let expected = Valid::from(vec![]);
        assert_eq!(valid, expected);
        assert!(valid.is_fail());
    }
}
