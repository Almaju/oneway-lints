# `oneway::no_if_else`

**Severity:** warn
**Enforced by:** `oneway_lints` (dylint)

Prefer `match` over `if`/`else` chains. Match is more explicit, forces you to handle all cases (exhaustiveness checking), and the arms can be sorted ([`unsorted_match_arms`](unsorted_match_arms.md)).

## Rationale

`if`/`else` chains grow silently — a new role, a new enum variant, a new edge case gets one more `else if` and the structure rots. A `match` over an enum or `Ordering` turns "did you cover everything?" into a compiler error, so adding a variant somewhere upstream surfaces every site that needs an update. Match arms are also easier to scan: each pattern is on the left, each result on the right, with no nested condition syntax.

## ❌ Bad

```rust
fn classify(n: i32) -> &'static str {
    if n < 0 {
        "negative"
    } else if n == 0 {
        "zero"
    } else {
        "positive"
    }
}
```

## ✅ Good

```rust
fn classify(n: i32) -> &'static str {
    match n.cmp(&0) {
        Ordering::Equal => "zero",
        Ordering::Greater => "positive",
        Ordering::Less => "negative",
    }
}
```

## ❌ Bad

```rust
fn describe(user: &User) -> String {
    if user.is_admin() {
        format!("Admin: {}", user.name())
    } else if user.is_moderator() {
        format!("Mod: {}", user.name())
    } else {
        format!("User: {}", user.name())
    }
}
```

## ✅ Good

```rust
fn describe(user: &User) -> String {
    match user.role() {
        Role::Admin => format!("Admin: {}", user.name()),
        Role::Moderator => format!("Mod: {}", user.name()),
        Role::User => format!("User: {}", user.name()),
    }
}
```
