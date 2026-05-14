# `oneway::subject_first_param`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

Every function must take its **subject** as the first parameter, and the
subject must actually be used. The only allowed shapes are:

- `fn name()` — zero params (entry point, no-arg constructor, pure value)
- `fn name(self)` — method, no extra input
- `fn name(self, param)` — method, one extra input

Anything else is rejected: free functions with parameters, methods with
more than one non-self param, **and methods that declare `self` but never
reference it in the body** — those are misplaced and belong as an
extension trait on whichever type the body is actually working with.

## Rationale

We prefer organising code as **methods on types**, not as free functions
that take values and operate on them. A method makes the subject of the
operation (`self`) explicit, and the method's name describes what the
subject is doing — the call site reads as `account.transfer(amount)`
rather than `transfer(account, amount)`. The subject is always in the same
slot, so reading a codebase becomes a matter of finding the right type
rather than guessing which utility module a free function lives in.

The single-extra-input cap pushes the same discipline at the input side:
if you find yourself wanting three or four parameters, the inputs are a
domain concept that deserves a struct or newtype. `transfer(from, to,
amount, memo)` becomes `wallet.transfer(Transfer { … })` — every input
field gets a name at the call site, every conceptual slot gets a real
type, and adding or reordering inputs doesn't ripple through callers.

For cross-cutting capabilities on foreign types, **use traits**. That's
the legitimate escape hatch: when you need to extend a type you don't own
with new behaviour, define a trait and implement it on the foreign type.
The lint skips trait *impl* method bodies (the signature is fixed by the
trait), but it does still apply to trait *declarations* — your own
traits should follow the same shape.

## ❌ Bad — free functions with parameters

```rust
fn send_email(to: &str, from: &str, subject: &str, body: &str) { ... }

fn classify(score: i32) -> Rank { ... }
```

## ✅ Good — methods on types

```rust
impl Email {
    fn send(&self) { ... }
}

impl Score {
    fn classify(&self) -> Rank { ... }
}
```

## ❌ Bad — methods with multiple non-self params

```rust
impl Wallet {
    fn transfer(&self, to: AccountId, amount: Amount, memo: Memo) { ... }
}
```

## ✅ Good — wrap inputs in a struct

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

## ❌ Bad — `self` declared but never used

```rust
impl NoComments {
    fn is_local_path(&self, path: &Path) -> bool {
        !path.to_string_lossy().contains("/.cargo/")
    }
}
```

The method body operates on `path`, not on `self`. The `&self` is dead
weight — a syntactic trick to satisfy the "subject first" rule without
following its spirit. The real subject is `Path`.

## ✅ Good — extension trait on the actual subject

```rust
trait PathExt {
    fn is_local_source(&self) -> bool;
}

impl PathExt for Path {
    fn is_local_source(&self) -> bool {
        !self.to_string_lossy().contains("/.cargo/")
    }
}
```

Now the call site reads `path.is_local_source()` — the subject is right
there in the first position, and the method body actually references it.

## ✅ Good — `fn name()` is fine

Zero-argument free functions are allowed because they have no inputs to
mis-order:

```rust
fn main() { ... }

fn default_config() -> Config { ... }
```

## Exceptions

The lint skips:

- **`extern "C"` declarations** (`FnCtxt::Foreign`) — signatures are
  fixed by the C ABI.
- **Trait `impl` method bodies** — signatures are constrained by the
  trait declaration. Fix the trait, not every impl.
- **Constructor-style associated functions** — any fn whose return type
  mentions `Self` (e.g. `fn new() -> Self`, `fn connect(name, config)
  -> Result<Self, _>`, `fn parse(s) -> Option<Self>`). The instance
  doesn't exist yet so `self` can't be first; the carve-out is
  required for the rule to coexist with idiomatic Rust constructors.

If you genuinely need a free function with parameters (rare — usually a
sign the design is missing a type), reach for `#[allow(subject_first_param)]`
on the specific function and leave a comment explaining why.
