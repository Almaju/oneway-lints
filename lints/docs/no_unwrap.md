# `oneway::no_unwrap`

**Severity:** deny
**Enforced by:** `clippy::unwrap_used` + `clippy::expect_used`

Never use `.unwrap()` or `.expect()` in non-test code. Use `?` or explicit `match`.

## Rationale

`unwrap` is a `panic!` in disguise: every call site is a latent runtime crash that fires the first time the input shape isn't what the author assumed. Returning the error via `?` or matching on it surfaces the failure in the type system, where callers are forced to either handle it or propagate it explicitly. Test code is exempt because a panicking test *is* a failing test — that's the intended behaviour there.

## ❌ Bad

```rust
fn read_config() -> Config {
    let content = std::fs::read_to_string("config.toml").unwrap();
    toml::from_str(&content).expect("invalid config")
}
```

## ✅ Good

```rust
fn read_config() -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string("config.toml")?;
    Ok(toml::from_str(&content)?)
}
```
