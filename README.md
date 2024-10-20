## Valid - Composable Validations with Error Accumulation in Rust

**Valid** is a Rust library that provides a powerful and flexible way to perform validations that can accumulate multiple errors. It allows you to compose multiple validation steps, collect all the errors that occur, and provide detailed error tracing. This library is particularly useful when you need to validate complex data structures and want to provide comprehensive error feedback to the user.

## Table of Contents

- [Valid - Composable Validations with Error Accumulation in Rust](#valid---composable-validations-with-error-accumulation-in-rust)
- [Table of Contents](#table-of-contents)
- [Features](#features)
- [Getting Started](#getting-started)
  - [Installation](#installation)
- [Usage](#usage)
  - [Creating Valid Instances](#creating-valid-instances)
  - [Composing Validations](#composing-validations)
  - [Collecting Errors](#collecting-errors)
  - [Tracing Errors](#tracing-errors)
- [Examples](#examples)
  - [Basic Validation](#basic-validation)
  - [Composing Multiple Validations](#composing-multiple-validations)
  - [Accumulating Errors from Iterators](#accumulating-errors-from-iterators)
  - [Adding Error Traces](#adding-error-traces)
- [API Overview](#api-overview)
  - [Valid\<A, E, T\>](#valida-e-t)
  - [Validator Trait](#validator-trait)
  - [Cause\<E, T\>](#causee-t)
- [Contributing](#contributing)

## Features

- **Composable Validations**: Combine multiple validations using methods like `and`, `zip`, and `and_then`.
- **Error Accumulation**: Collect all errors instead of failing fast, providing comprehensive feedback.
- **Error Tracing**: Attach contextual information to errors to help identify where and why they occurred.
- **Flexible API**: A rich set of methods to manipulate and transform validation results.

## Getting Started

### Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
valid = "0.1.0"
```

Then include it in your project:

```rust
use valid::{Valid, Validator, Cause};
```

## Usage

### Creating Valid Instances

Use `Valid::succeed` to create a successful validation:

```rust
let valid_value = Valid::succeed(42);
```

Use `Valid::fail` to create a failed validation:

```rust
let error_value = Valid::<i32, &str, ()>::fail("Validation error");
```

### Composing Validations

Combine validations using `and` or `zip`:

```rust
let valid1 = Valid::succeed(10);
let valid2 = Valid::succeed(20);

let combined = valid1.and(valid2); // Succeeds with 20
```

Use `and_then` to chain validations that depend on previous results:

```rust
let result = valid1.and_then(|value| Valid::succeed(value * 2));
```

### Collecting Errors

When multiple validations fail, `Valid` collects all errors:

```rust
let fail1 = Valid::<(), &str, ()>::fail("Error 1");
let fail2 = Valid::<(), &str, ()>::fail("Error 2");

let combined = fail1.zip(fail2); // Fails with ["Error 1", "Error 2"]
```

### Tracing Errors

Add contextual information to errors using `trace`:

```rust
let result = Valid::<(), &str, &str>::fail("Error")
    .trace("Function A")
    .trace("Processing item 1");
```

## Examples

### Basic Validation

```rust
use valid::{Valid, Validator};

fn validate_age(age: i32) -> Valid<i32, &'static str, ()> {
    if age >= 18 {
        Valid::succeed(age)
    } else {
        Valid::fail("Age must be at least 18")
    }
}

let age = validate_age(20);
assert!(age.is_succeed());

let invalid_age = validate_age(16);
assert!(invalid_age.is_fail());
```

### Composing Multiple Validations

```rust
fn validate_username(username: &str) -> Valid<&str, &'static str, ()> {
    if username.len() >= 3 {
        Valid::succeed(username)
    } else {
        Valid::fail("Username must be at least 3 characters")
    }
}

fn validate_password(password: &str) -> Valid<&str, &'static str, ()> {
    if password.len() >= 8 {
        Valid::succeed(password)
    } else {
        Valid::fail("Password must be at least 8 characters")
    }
}

let username = validate_username("user");
let password = validate_password("pass123");

let combined = username.zip(password);
if combined.is_fail() {
    let errors = combined.to_result().unwrap_err();
    for cause in errors {
        println!("Error: {}", cause.error);
    }
}
```

### Accumulating Errors from Iterators

```rust
let inputs = vec![2, 4, 6, 7];
let result = Valid::from_iter(inputs, |num| {
    if num % 2 == 0 {
        Valid::succeed(num)
    } else {
        Valid::fail(format!("Number {} is not even", num))
    }
});

match result.to_result() {
    Ok(even_numbers) => println!("All numbers are even: {:?}", even_numbers),
    Err(errors) => {
        for cause in errors {
            println!("Error: {}", cause.error);
        }
    }
}
```

### Adding Error Traces

```rust
let result = Valid::<(), &str, &str>::fail("Invalid data")
    .trace("Parsing configuration")
    .trace("Line 42");

if let Err(errors) = result.to_result() {
    for cause in errors {
        println!("Error: {}", cause.error);
        println!("Trace: {:?}", cause.trace);
    }
}
```

## API Overview

### Valid\<A, E, T\>

A struct representing the result of a validation operation that can succeed with a value of type `A` or fail with an error of type `E`. It also includes a trace value of type `T`.

- `Valid::succeed(a: A) -> Valid<A, E, T>`: Creates a successful validation.
- `Valid::fail(e: E) -> Valid<A, E, T>`: Creates a failed validation with an error.
- `Valid::from(errors: Vec<Cause<E, T>>) -> Valid<A, E, T>`: Creates a failed validation with multiple errors.

### Validator Trait

Provides methods for working with validations:

- `map(self, f: impl FnOnce(A) -> B) -> Valid<B, E, T>`: Transforms the success value.
- `and(self, other: Valid<B, E, T>) -> Valid<B, E, T>`: Composes two validations, returning the second if both succeed.
- `zip(self, other: Valid<B, E, T>) -> Valid<(A, B), E, T>`: Combines two validations into one with both values.
- `and_then(self, f: impl FnOnce(A) -> Valid<B, E, T>) -> Valid<B, E, T>`: Chains validations that depend on previous results.
- `trace(self, trace: T) -> Valid<A, E, T>`: Adds context to errors.

### Cause\<E, T\>

A struct representing an error cause with an error value of type `E` and a trace value of type `T`.

- `Cause::new(error: E) -> Cause<E, T>`: Creates a new error cause.
- `trace(self, trace: T) -> Self`: Adds trace information to the cause.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on [GitHub](https://github.com/yourusername/valid).

Feel free to explore the library and use it in your projects. If you have any questions or suggestions, please don't hesitate to reach out.
