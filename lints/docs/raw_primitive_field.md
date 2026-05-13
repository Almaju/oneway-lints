# `oneway::raw_primitive_field`

**Severity:** warn
**Enforced by:** `oneway_lints` (dylint)

Struct fields should use newtypes instead of raw primitives (`i32`, `i64`, `u64`, `f64`, `String`, `bool`).

## Rationale

Two reasons. **Self-documentation**: `UserId` carries meaning at every use site that `u64` doesn't — you don't have to chase the field name back to the struct to understand what the value represents. **Type safety**: when two fields share a primitive (e.g. two `u64` IDs, two `String` addresses), the compiler can't catch a swap; newtypes turn that swap into a type error.

## ❌ Bad

```rust
struct Order {
    price: f64,
    quantity: u32,
    user_id: u64,
}
```

## ✅ Good

```rust
struct Price(f64);
struct Quantity(u32);
struct UserId(u64);

struct Order {
    price: Price,
    quantity: Quantity,
    user_id: UserId,
}
```

## Autofix

`cargo oneway lint --fix` introduces a newtype for each offending field: the
field name is converted from `snake_case` to `PascalCase` (`user_id` →
`UserId`) and the new `struct Newtype(primitive);` is inserted right after
the parent struct. Visibility is copied from the field so the newtype is
constructible from wherever the parent struct was. Call sites that
previously passed raw values (`Order { price: 9.99, … }`) will break and
must be wrapped manually (`Order { price: Price(9.99), … }`) — `cargo
oneway lint --fix` runs `cargo fix --broken-code` so the autofix lands
even though the intermediate code doesn't typecheck.

Fields behind a reference (`&str`, `&u32`) are diagnostic-only — `&str`
can't be wrapped without changing the field's storage shape.
