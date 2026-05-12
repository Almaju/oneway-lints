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
