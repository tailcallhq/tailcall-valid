# tailcall-valid

This crate helps to collect all possible errors instead of returning the first error encountered. This is useful when you want to know all the errors in a single go.

`Valid` could be initiated with an `Option`, `Result`, success, or a failure.

## Examples

### From success

```rust
use tailcall_valid::*;

fn main() {
    let result: Valid<i32, &str> = Valid::succeed(1);
    assert_eq!(result, Valid::succeed(1));
}
```

### From failure

```rust
use tailcall_valid::*;

fn main() {
    let err = "Expected a value";
    let result: Valid<i32, &str> = Valid::fail(err);
    assert_eq!(result, Valid::from_vec_cause(vec![Cause::new(err)]));
}
```

### From `Option`

```rust
use tailcall_valid::*;

fn main() {
    // Case when Option is None
    let err = "Expected a value";
    let option: Option<i32> = None;
    let result = Valid::from_option(option, err);
    assert_eq!(result, Valid::from_vec_cause(vec![Cause::new(err)]));

    // Case when Option is Some
    let option: Option<i32> = Some(1);
    let result = Valid::from_option(option, err);
    assert_eq!(result, Valid::succeed(1));
}
```

### From `Result`

```rust
use tailcall_valid::*;

fn main() {
    // Case when Result is Err
    let err = "Expected a value";
    let result: Result<i32, &str> = Err(err);
    let result = result.map_err(ValidationError::new);
    let result = Valid::from(result);
    assert_eq!(result, Valid::from_vec_cause(vec![Cause::new(err)]));

    // Case when Result is Ok
    let result: Result<i32, &str> = Ok(1);
    let result = result.map_err(ValidationError::new);
    let result = Valid::from(result);
    assert_eq!(result, Valid::succeed(1));
}
```
