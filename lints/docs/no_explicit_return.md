# `oneway::no_explicit_return`

**Severity:** warn
**Enforced by:** `clippy::needless_return`

Don't use the `return` keyword when the last expression in the block serves the same purpose.

## Rationale

Rust is expression-oriented: a function body's last expression *is* its return value. Spelling that out with `return` is redundant noise and creates two ways to do the same thing within a single function (mixed-style bodies are common in code that's been edited by multiple authors). Reserving `return` for genuine early exits keeps the trailing expression as the unambiguous "happy path" of the function.

## ❌ Bad

```rust
fn is_valid(age: u32) -> bool {
    if age >= 18 && age <= 120 {
        return true;
    }
    return false;
}
```

## ✅ Good

```rust
fn is_valid(age: u32) -> bool {
    age >= 18 && age <= 120
}
```
