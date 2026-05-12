# `oneway::too_many_params`

**Severity:** deny
**Enforced by:** `clippy::too_many_arguments` (configured via `clippy.toml`: `too-many-arguments-threshold = 2`)

Functions must have at most 2 parameters (including `&self`). The shape of a function is one of:

- `fn name()` — 0 params
- `fn name(input: T)` or `fn name(&self)` — single value
- `fn name(&self, input: T)` — receiver + one input

Anything more must be packed into a struct.

## Rationale

Long argument lists invite call-site bugs: `transfer(from, to, amount)` is easy to call with `from` and `to` swapped, and the compiler will not catch it. Packing parameters into a struct gives every field a name at the call site (`Transfer { from, to, amount }`) and gives each conceptually-named slot a real type. Long signatures also signal a function that is doing too much — the cap forces you to either narrow the scope or introduce a proper input type.

## ❌ Bad

```rust
fn send_email(to: &str, from: &str, subject: &str, body: &str) {
    // ...
}
```

## ✅ Good

```rust
struct Email {
    body: String,
    from: String,
    subject: String,
    to: String,
}

fn send_email(email: &Email) {
    // ...
}
```

## ❌ Bad — methods too

```rust
impl Wallet {
    fn transfer(&self, to: &Account, amount: Amount, memo: &str) { ... }
}
```

## ✅ Good

```rust
struct Transfer {
    amount: Amount,
    memo: Memo,
    to: AccountId,
}

impl Wallet {
    fn transfer(&self, transfer: Transfer) { ... }
}
```
