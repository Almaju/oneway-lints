# `oneway::one_constructor_name`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

Constructors must be named `new`. Not `create`, `build`, `init`, `make`, `construct`, or `from_*` (except `From` trait impls).

## Rationale

A single canonical constructor name means readers and IDEs never have to guess: typing `Foo::` and finding `new` is faster than recalling which crate uses `create` vs `init` vs `build`. The Rust ecosystem already converged on `new` for this — the lint just enforces what's already idiomatic. Variant constructors (`from_str`, `from_bytes`) are fine when there's a real `From<T>` semantic; what's banned is using `from_*` as a synonym for `new`.

## ❌ Bad

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

## ✅ Good

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
