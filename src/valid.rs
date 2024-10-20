use super::append::Append;
use super::Cause;

#[derive(Debug, PartialEq)]
pub struct Valid<A, E, T>(Result<A, Vec<Cause<E, T>>>);

pub trait Validator<A, E, T>: Sized {
    fn map<A1>(self, f: impl FnOnce(A) -> A1) -> Valid<A1, E, T> {
        Valid(self.to_result().map(f))
    }

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

    fn is_succeed(&self) -> bool;

    fn is_fail(&self) -> bool;

    fn and<A1>(self, other: Valid<A1, E, T>) -> Valid<A1, E, T> {
        self.zip(other).map(|(_, a1)| a1)
    }

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

    fn fuse<A1>(self, other: Valid<A1, E, T>) -> Fusion<(A, A1), E, T> {
        Fusion(self.zip(other))
    }

    fn trace(self, trace: T) -> Valid<A, E, T>
    where
        T: Clone,
    {
        let valid = self.to_result();
        if let Err(error) = valid {
            return Valid(Err(error
                .into_iter()
                .map(|cause| cause.trace(trace.clone()))
                .collect()));
        }

        Valid(valid)
    }

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

    fn to_result(self) -> Result<A, Vec<Cause<E, T>>>;

    fn and_then<B>(self, f: impl FnOnce(A) -> Valid<B, E, T>) -> Valid<B, E, T> {
        match self.to_result() {
            Ok(a) => f(a),
            Err(e) => Valid(Err(e)),
        }
    }

    fn unit(self) -> Valid<(), E, T> {
        self.map(|_| ())
    }

    fn some(self) -> Valid<Option<A>, E, T> {
        self.map(Some)
    }

    fn map_to<B>(self, b: B) -> Valid<B, E, T> {
        self.map(|_| b)
    }
    fn when(self, f: impl FnOnce() -> bool) -> Valid<(), E, T> {
        if f() {
            self.unit()
        } else {
            Valid::succeed(())
        }
    }
}

impl<A, E, T> Valid<A, E, T> {
    pub fn fail(e: E) -> Valid<A, E, T> {
        Valid(Err((vec![Cause {
            error: e,
            trace: Default::default(),
        }])
        .into()))
    }

    pub fn fail_at(error: E, trace: T) -> Valid<A, E, T>
    where
        E: std::fmt::Debug,
    {
        let cause = Cause::new(error).trace(trace);
        Valid(Err((vec![cause]).into()))
    }

    pub fn from(error: Vec<Cause<E, T>>) -> Self {
        Valid(Err(error.into()))
    }

    pub fn succeed(a: A) -> Valid<A, E, T> {
        Valid(Ok(a))
    }

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

    pub fn from_option(option: Option<A>, e: E) -> Valid<A, E, T> {
        match option {
            Some(a) => Valid::succeed(a),
            None => Valid::fail(e),
        }
    }

    pub fn none() -> Valid<Option<A>, E, T> {
        Valid::succeed(None)
    }
}

impl<A, E, T> From<Cause<E, T>> for Valid<A, E, T> {
    fn from(value: Cause<E, T>) -> Self {
        Valid(Err(vec![value]))
    }
}

impl<A, E, T> From<Vec<Cause<E, T>>> for Valid<A, E, T> {
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

pub struct Fusion<A, E, T>(Valid<A, E, T>);
impl<A, E, T> Fusion<A, E, T> {
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
    fn from(value: Result<A, Cause<E, T>>) -> Self {
        match value {
            Ok(a) => Valid::succeed(a),
            Err(e) => Valid(Err(vec![e])),
        }
    }
}

impl<A, E, T> From<Fusion<A, E, T>> for Valid<A, E, T> {
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
    use super::Cause;
    use crate::{Valid, Validator};

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
            .trace("A".into())
            .trace("B".into())
            .trace("C".into());

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
}
