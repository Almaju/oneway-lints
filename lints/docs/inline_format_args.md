# `oneway::inline_format_args`

**Severity:** deny
**Enforced by:** `clippy::uninlined_format_args`

Use inline variable capture in format strings. Don't pass variables as separate arguments when the captured form works.

## Rationale

Inline captures keep each value adjacent to its placeholder, so the format string reads left-to-right with no comma-counting. Positional arguments make slot/value pairing a manual step for the reader and an easy source of off-by-one bugs when an argument is added or removed.

## ❌ Bad

```rust
let message = format!("Hello, {}! You are {} years old.", name, age);
log::info!("Processing order {} for user {}", order_id, user_id);
```

## ✅ Good

```rust
let message = format!("Hello, {name}! You are {age} years old.");
log::info!("Processing order {order_id} for user {user_id}");
```
