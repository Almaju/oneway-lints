# `oneway::no_turbofish`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

Don't use turbofish syntax (`::<>`). Annotate the binding instead — it's easier to read and easier to skim.

## Rationale

A turbofish buries the result type inside the expression, often at the end of a long chain (`...collect::<Vec<_>>()`); a binding annotation puts the type information next to the name (`let xs: Vec<_> = ...`), which is where the reader looks first. Binding annotations also stay put when the expression is refactored — the type doesn't have to move with the call site. Reserving one location for type ascription removes the "where do I put the type?" decision every time inference needs a nudge.

## ❌ Bad

```rust
let names = users.iter().map(|u| u.name.clone()).collect::<Vec<String>>();
let parsed = "42".parse::<i32>()?;
```

## ✅ Good

```rust
let names: Vec<String> = users.iter().map(|u| u.name.clone()).collect();
let parsed: i32 = "42".parse()?;
```
