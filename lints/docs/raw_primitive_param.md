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

## Autofix

`cargo oneway lint --fix` inserts a newtype declaration immediately before
the function and rewrites the param's type. Param name is converted from
`snake_case` to `PascalCase` for the type identifier. Body uses of the
param (`from + 1`) and existing call sites both stop compiling and have to
be updated to wrap / unwrap manually — the autofix lands anyway because
`cargo oneway lint --fix` passes `--broken-code` to `cargo fix`.

Skipped:

- **Trait impl methods** — signatures are dictated by the trait (you can't
  rewrite `FromStr::from_str(s: &str)` to take a newtype). The lint skips
  trait impl method bodies entirely; if the trait belongs to you and the
  parameter shape is wrong, fix it at the trait declaration.
- **Reference params** (`&str`, `&u32`) — diagnostic-only, no autofix.
  The newtype shape doesn't transfer cleanly through indirection.

Autofix is also withheld for inherent impl methods and trait methods —
inserting the newtype declaration before the fn span would land it inside
the `impl` / `trait` block, which isn't valid Rust. The diagnostic still
fires; move the type outside the impl by hand.
