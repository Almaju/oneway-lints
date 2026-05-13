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

## Autofix

`cargo oneway lint --fix` rewrites the entire chain (only the outermost
warning per chain is now emitted) as a guard-based `match`:

```rust
match () {
    _ if n < 0 => { "negative" },
    _ if n == 0 => { "zero" },
    _ => { "positive" },
}
```

The rewrite preserves semantics but doesn't promote the discriminant —
`match () { _ if cond => … }` is a uniform mechanical fit. The good
versions above (matching on `Ordering` or `Role`) are the intended next
step; treat the autofix as a starting point that gets you out of
`if/else` syntactically, then tighten manually.

Skipped when any condition in the chain uses `if let` (the pattern can't
flatten into a guard) or when the chain has no final `else` (match must
be exhaustive).
