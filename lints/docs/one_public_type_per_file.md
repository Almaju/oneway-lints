# `oneway::one_public_type_per_file`

**Severity:** warn
**Enforced by:** `oneway_lints` (dylint)

Each file should export at most one primary public type (struct/enum). Related types (newtypes, error types) are fine as supporting cast.

## Rationale

Forces module boundaries to follow the type system: each file owns one concept, and the filename is the search key. Finding a type becomes a matter of finding a filename rather than greping for its declaration, and the size of any one file stays proportional to the complexity of one concept. Newtypes and error types living alongside their owner is the deliberate exception — they're part of the same concept.

## ❌ Bad — three unrelated types in one file

```rust
pub struct User { ... }
pub struct Order { ... }
pub struct Product { ... }
```

## ✅ Good — split by primary type, supporting newtypes live with their owner

```
// user.rs
pub struct User { ... }
pub struct UserId(u64);

// order.rs
pub struct Order { ... }
pub struct OrderId(u64);

// product.rs
pub struct Product { ... }
pub struct ProductId(u64);
```
