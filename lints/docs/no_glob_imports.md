# `oneway::no_glob_imports`

**Severity:** deny
**Enforced by:** `clippy::wildcard_imports`

No wildcard imports. Every imported symbol must be named explicitly.

## Rationale

`use foo::*` makes the symbol set at the top of a file invisible: you can't grep for where `Something` came from, and a reader can't tell which crate owns a given name without IDE help. It also creates upstream-fragile code — adding a new public item to `foo` can silently shadow a local binding or introduce ambiguity that breaks the build for no good reason. Naming every import keeps the import block honest and the file stable against unrelated upstream changes.

## ❌ Bad

```rust
use std::collections::*;
use crate::models::*;
```

## ✅ Good

```rust
use std::collections::HashMap;
use crate::models::User;
```
