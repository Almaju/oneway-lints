# `oneway::mod_after_use`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

Every `mod` declaration in a module must appear before any `use` statement. `cargo fmt` already orders `use` statements alphabetically, but it does not enforce the mod/use split — this lint does.

## Rationale

`mod` declarations define what *exists* in the crate; `use` statements bring symbols into scope. Reading top-to-bottom, structure should come before consumption — so a reader opening any file knows the module's shape before seeing what it borrows from elsewhere. Interleaving the two makes the file's surface impossible to skim.

## ❌ Bad

```rust
use std::collections::HashMap;

mod parser;

use std::collections::BTreeMap;

mod printer;
```

## ✅ Good

```rust
mod parser;
mod printer;

use std::collections::BTreeMap;
use std::collections::HashMap;
```
