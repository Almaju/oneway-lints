# `oneway::unsorted_match_arms`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

Match arms must be sorted by pattern text. The wildcard `_` arm must always be last.

## Rationale

Same logic as [struct fields](unsorted_struct_fields.md) and [enum variants](unsorted_enum_variants.md): arm order doesn't affect behaviour (the compiler picks the first matching arm, but disjoint patterns make order irrelevant), so pinning a canonical order kills debates and keeps diffs minimal. Forcing `_` last reinforces the "exhaust the named cases, then catch the rest" reading.

## ❌ Bad

```rust
match color {
    Color::Red => "red",
    Color::Blue => "blue",
    Color::Green => "green",
}
```

## ✅ Good

```rust
match color {
    Color::Blue => "blue",
    Color::Green => "green",
    Color::Red => "red",
}
```
