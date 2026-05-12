# `oneway::unsorted_struct_fields`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

Struct fields must be in alphabetical order.

## Rationale

Field order has no semantic meaning, so freezing it alphabetically removes bikeshedding ("logical grouping" debates never end), keeps diffs minimal when fields are added or removed, and lets readers locate a field by binary-searching the source instead of scanning.

## ❌ Bad

```rust
struct User {
    name: String,
    email: String,
    age: u32,
}
```

## ✅ Good

```rust
struct User {
    age: u32,
    email: String,
    name: String,
}
```
