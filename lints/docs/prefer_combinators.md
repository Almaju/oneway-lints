# `oneway::prefer_combinators`

**Severity:** warn
**Enforced by:** `clippy::single_match` + `clippy::manual_map` + `clippy::manual_unwrap_or`

Use `Option`/`Result` combinators instead of `match` for simple transforms. If you're just mapping, filtering, or providing a default, use the combinator.

## Rationale

Combinators name the intent (`map`, `unwrap_or`, `and_then`) in a single word — the reader sees what's happening without parsing the two arms of a `match` to confirm "ah, this is just a map". A `match` should signal "the two arms do meaningfully different work"; using one for `Some(x) => x, None => default` dilutes that signal and adds line noise. Combinators also chain cleanly into longer pipelines where the case-by-case form would balloon.

## ❌ Bad

```rust
let display_name = match user.nickname {
    Some(nick) => nick,
    None => user.name.clone(),
};

let upper = match value {
    Some(s) => Some(s.to_uppercase()),
    None => None,
};

let count = match result {
    Ok(items) => items.len(),
    Err(_) => 0,
};
```

## ✅ Good

```rust
let display_name = user.nickname
    .unwrap_or_else(|| user.name.clone());

let upper = value.map(|s| s.to_uppercase());

let count = result
    .map(|items| items.len())
    .unwrap_or(0);
```
