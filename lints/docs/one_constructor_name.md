# `oneway::one_constructor_name`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

Constructors must not use the near-synonyms `create`, `build`, `init`, `make`, or `construct`. Use `new` for the canonical constructor, and pick descriptive names (`from_string`, `with_capacity`, role discriminators like `user`/`system`) for the variants.

## Rationale

`create` / `build` / `init` / `make` / `construct` all mean the same thing as `new` but differ across crates — readers and IDEs have to guess which spelling a given API uses. The Rust ecosystem converged on `new` for the canonical zero-or-one-input constructor; the lint enforces that.

It does *not* forbid descriptive constructor names. A type often has multiple genuinely different ways to be built — `UserId::new()` (random UUID) vs `UserId::from_string(s)` (parse), `Vec::new()` vs `Vec::with_capacity(n)`, `Message::user(content)` vs `Message::system(content)`. Each of those names tells the reader something the call site can't otherwise communicate. Collapsing them into one `new(args...)` would push that information into argument lists — worse for readability.

The line: ban only the names that *pretend* to add information but don't (synonyms for `new`); allow names that actually discriminate (different inputs, different semantic roles).

## ❌ Bad — synonyms for `new`

```rust
impl Server {
    fn create(config: ServerConfig) -> Self { ... }
}

impl Database {
    fn init(url: &str) -> Self { ... }
}

impl HttpClient {
    fn build() -> Self { ... }
}
```

## ✅ Good — use `new`

```rust
impl Server {
    fn new(config: ServerConfig) -> Self { ... }
}

impl Database {
    fn new(url: &str) -> Self { ... }
}

impl HttpClient {
    fn new() -> Self { ... }
}
```

## ✅ Good — multiple descriptive constructors

```rust
impl UserId {
    fn new() -> Self { Self(Uuid::new_v4()) }
    fn from_string(s: &str) -> Result<Self, ParseError> { ... }
}

impl Message {
    fn user(content: String) -> Self { ... }
    fn system(content: String) -> Self { ... }
    fn assistant(content: String) -> Self { ... }
}
```
