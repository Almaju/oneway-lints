# `oneway::unsorted_derives`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

`#[derive(...)]` attributes must list traits in alphabetical order.

## Rationale

The derive list has no semantic meaning to the compiler — the macros expand in textual order but produce the same trait impls regardless. Pinning it alphabetically keeps diffs and reviews quiet, and makes it obvious at a glance whether a given trait (e.g. `Serialize`) is in the list.

## ❌ Bad

```rust
#[derive(Debug, Clone, Serialize, PartialEq)]
struct User {
    name: Name,
}
```

## ✅ Good

```rust
#[derive(Clone, Debug, PartialEq, Serialize)]
struct User {
    name: Name,
}
```
