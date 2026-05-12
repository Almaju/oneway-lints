# `oneway::raw_primitive_param`

**Severity:** warn
**Enforced by:** `oneway_lints` (dylint)

Function parameters should use newtypes instead of raw primitives.

## Rationale

Same reasoning as [`raw_primitive_field`](raw_primitive_field.md), applied to function signatures. A call like `transfer(123, 456, 100.0)` gives the compiler nothing to validate — swapping `from` and `to` will compile and ship. `transfer(from: AccountId, to: AccountId, amount: Amount)` makes the swap a type error, and the signature becomes self-documenting without needing comments or parameter labels.

## ❌ Bad

```rust
fn transfer(from: u64, to: u64, amount: f64) {
    // Easy to accidentally swap `from` and `to`
}
```

## ✅ Good

```rust
fn transfer(from: AccountId, to: AccountId, amount: Amount) {
    // Types prevent misuse
}
```
