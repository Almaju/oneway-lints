# `oneway::unsorted_enum_variants`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

Enum variants must be in alphabetical order.

## Rationale

Variant declaration order has no runtime effect, so pinning it alphabetically prevents arbitrary churn — every reorder is a real change, not a style nit — and gives reviewers one canonical layout to scan.

## ❌ Bad

```rust
enum Color {
    Red,
    Blue,
    Green,
}
```

## ✅ Good

```rust
enum Color {
    Blue,
    Green,
    Red,
}
```
