# `oneway::no_panic`

**Severity:** deny
**Enforced by:** `clippy::panic` + `clippy::todo` + `clippy::unimplemented` + `clippy::unreachable`

Never use `panic!`, `todo!`, `unimplemented!`, or `unreachable!` in non-test code. Return `Result` or handle the case.

## Rationale

Same reasoning as [`no_unwrap`](no_unwrap.md): cases you can't currently handle belong in the type system as `Result`, not as runtime crashes. `todo!` and `unimplemented!` are especially dangerous in non-test code — they look like placeholders during development and then ship to production, where the first user to hit them brings down the process. If a path is genuinely unreachable, refactor the types until the compiler agrees; if it isn't, it needs a real error variant.

## ❌ Bad

```rust
fn divide(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        panic!("division by zero");
    }
    a / b
}
```

## ✅ Good

```rust
fn divide(a: f64, b: f64) -> Result<f64, DivisionError> {
    match b == 0.0 {
        true => Err(DivisionError::DivideByZero),
        false => Ok(a / b),
    }
}
```
