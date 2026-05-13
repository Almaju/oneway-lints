# `oneway::one_public_type_per_file`

**Severity:** warn
**Enforced by:** `oneway_lints` (dylint)

Each file should export at most one primary public type (struct/enum). Related types (newtypes, error types) are fine as supporting cast.

## Rationale

Forces module boundaries to follow the type system: each file owns one concept, and the filename is the search key. Finding a type becomes a matter of finding a filename rather than greping for its declaration, and the size of any one file stays proportional to the complexity of one concept. Newtypes and error types living alongside their owner is the deliberate exception — they're part of the same concept.

## ❌ Bad — three unrelated types in one file

```rust
pub struct User { ... }
pub struct Order { ... }
pub struct Product { ... }
```

## ✅ Good — split by primary type, supporting newtypes live with their owner

```
// user.rs
pub struct User { ... }
pub struct UserId(u64);

// order.rs
pub struct Order { ... }
pub struct OrderId(u64);

// product.rs
pub struct Product { ... }
pub struct ProductId(u64);
```

## Autofix

`cargo oneway lint --fix` extracts each "extra" primary public type into its
own file alongside the parent. For `src/lib.rs` or `src/main.rs` siblings
land in the same directory (`src/<name>.rs`); for any other parent, the
extracted file goes inside a sibling directory matching the parent's stem
(`src/foo.rs` → `src/foo/<name>.rs`). The parent file gets
`<vis> mod <name>;` slotted in after any existing `mod` declarations and
`<vis> use <name>::<Type>;` slotted in before any existing `use` block (so
the `mod_after_use` invariant holds). Leading `#[derive(...)]` /
`#[doc = "..."]` attributes on the extracted type are moved with it.

The extracted file is **not** import-fixed — references to types that lived
in the parent file will fail to compile until you add `use` statements (or
`use super::*;`). This is the trade-off for not needing a full name
resolution pass; the compiler errors will tell you exactly what's missing.

If the destination file already exists, the extraction is skipped (the
diagnostic continues to fire) so unrelated content isn't clobbered.
