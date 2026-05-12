# `oneway::no_nested_functions`

**Severity:** warn
**Enforced by:** `oneway_lints` (dylint)

Don't define functions inside other functions. Extract them to module level.

## Rationale

Nested `fn` items can't capture surrounding state (that's what closures are for), so they offer nothing a top-level function doesn't. What they *do* cost: they hide from IDE outlines, inflate the visible length of the outer function, and can't be unit-tested in isolation. If a helper is worth naming, it's worth lifting out where it can be found, tested, and reused.

## ❌ Bad

```rust
fn process(items: &[Item]) -> Vec<Result> {
    fn transform(item: &Item) -> Result {
        // ...
    }
    items.iter().map(transform).collect()
}
```

## ✅ Good

```rust
fn transform(item: &Item) -> Result {
    // ...
}

fn process(items: &[Item]) -> Vec<Result> {
    items.iter().map(transform).collect()
}
```
