# Changelog

All notable changes to the `cargo-oneway` CLI and the bundled `oneway-lints` library.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). From v0.1.7 onward, the CLI version pins the lint library to the matching `vX.Y.Z` git tag — `cargo install cargo-oneway --version X.Y.Z` gives you exactly the rules from this section.

## Unreleased

### Added
- **Autofix for `raw_primitive_field`** — `cargo oneway lint --fix` introduces a newtype per offending field. Field name is converted snake_case → PascalCase for the type identifier, visibility is copied from the field, and the new `struct Name(primitive);` is appended after the parent struct (so any leading `#[derive(...)]` stays attached to the parent rather than the newtype). Call sites that passed raw values must wrap them manually afterward — the autofix relies on `cargo fix --broken-code` (passed by the CLI) so the rewrite lands even though the intermediate code doesn't typecheck. Fields behind a reference (`&str`, `&u32`) remain diagnostic-only — the newtype shape doesn't transfer cleanly through indirection.
- **Autofix for `raw_primitive_param`** — same shape as the field autofix, applied to free functions: newtype declaration is inserted immediately before the fn span, the param's primitive is replaced. Body uses of the param and call sites both break and must be rewrapped/unwrapped after the fix. Skipped for impl methods and trait methods (would insert a struct inside the impl/trait block) and for reference params.
- **Autofix for `one_public_type_per_file`** — `cargo oneway lint --fix` now runs an extraction pass after the normal rustc-suggestion-based fixes: each extra primary public type is moved to its own file (sibling for `lib.rs`/`main.rs`, `<parent_stem>/<name>.rs` for everything else), and the parent file gets `<vis> mod <name>;` + `<vis> use <name>::<Type>;` slotted in to preserve the `mod_after_use` invariant. Leading attributes (`#[derive(...)]`, doc comments) on the extracted type are moved with it. Extraction is skipped if the destination already exists. The extracted file is not import-fixed — references that lived in the parent will need `use` additions after the move; the compiler will point them out. Requires the new `serde_json` dependency on the CLI for parsing `cargo dylint`'s JSON diagnostic stream.
- **Autofix for `no_nested_functions`** — hoists the inner fn to module level by deleting it from the outer fn's body and reinserting after the outer fn's closing brace. Safe because Rust's nested `fn` items can't close over outer locals or generics; the only failure mode is a name collision at module scope. Skipped when the outer fn is an `impl` method or trait method — hoisting would land the new fn inside the impl block.
- **Autofix for `no_if_else`** — rewrites `if cond1 { a } else if cond2 { b } else { c }` chains into `match () { _ if cond1 => { a }, _ if cond2 => { b }, _ => { c }, }`. Lint now uses a Visitor pass so only the outermost `if` in a chain emits a warning (previously the lint fired once per `else if`, producing N warnings for one chain). Skipped when any condition is `if let` or when the chain has no final `else` (match exhaustiveness). The guard-based rewrite is mechanically correct but doesn't extract a discriminant; that's a manual follow-up.
- **Autofix for `one_constructor_name`** — renames a forbidden-name constructor to `new` and rewrites every `<Type>::<old_name>` call site in the crate. Crate-wide AST scan collects all candidate impl blocks and matching path expressions in two passes (gather candidates sorted by source position for determinism, then walk call sites). Applies only when the type has exactly one forbidden-name constructor AND no existing `new` method (otherwise the rename would collide or be ambiguous). Last-two-segment matching on call sites covers qualified paths. Rare false-positive risk if two unrelated types in different modules share the same simple name.

### Changed
- `cargo oneway lint --fix` now passes `--broken-code` to the underlying `cargo fix` so autofixes whose intermediate state doesn't typecheck (every newtype-wrapping or rename suggestion) actually land instead of being silently rolled back. The CLI also adds `serde_json` as a dependency for the extraction pass.

## [0.1.14] - 2026-05-12

### Added
- **Autofix for `type_derived_naming` (function parameters)** — `cargo oneway lint --fix` renames the parameter *and* every single-segment Path expression in the fn body that references it. Implementation pre-walks each fn body to collect path references and pat-binding counts per name. Skipped when the name appears as an inner pat-binding (`let`, `if let`, match arm) — that shadow would make a global rename point past the new binding. `let`-binding cases are diagnostic-only (their scope is "rest of enclosing block until shadowed", which we don't track precisely).

## [0.1.12] - 2026-05-12

### Added
- **Autofix for `unsorted_match_arms`** — `cargo oneway lint --fix` swaps non-wildcard arms to their sorted positions via a multi-part suggestion. Skipped when any arm has a guard (`pat if cond`) since guard arms can overlap with later patterns. Skipped when a wildcard arm is mis-positioned (the diagnostic still fires).
- **Autofix for `unsorted_impl_methods`** — full sort of methods by (group, name) via multi-part swap. Non-method associated items (consts, types) stay in place. The rule now computes the desired order once and emits a single diagnostic at the first divergence.
- **Autofix for `mod_after_use`** — one diagnostic per item list with a multi-part suggestion that rewrites all `mod`/`use` items into mods-first order. Other items (`fn`, `struct`, etc.) stay where they are.

## [0.1.11] - 2026-05-12

### Added
- **Autofix for `unsorted_enum_variants`** — multi-part swap of variants to their sorted positions. Skipped when variant order is load-bearing: derived `Ord` / `PartialOrd` / `Hash` (compared by declaration order), or any variant has an explicit discriminant. Derive detection probes the source text immediately before the item since `#[derive(...)]` isn't in `item.attrs` at the EarlyLintPass stage.

## [0.1.10] - 2026-05-12

### Added
- **Autofix for `unsorted_derives`** — `cargo oneway lint --fix` replaces `#[derive(...)]` with the alphabetically sorted version. Pure textual sort, no semantic risk.
- **Autofix for `unsorted_struct_fields`** — multi-part suggestion swaps each field's source range (including attributes and doc comments) to its sorted-rank counterpart. Skipped when the struct carries `#[repr(...)]` since field order is load-bearing for FFI and packed layouts.

## [0.1.8] - 2026-05-12

### Added
- `cargo oneway version` subcommand (also responds to `--version` / `-V`).

### Changed
- READMEs restructured: root is now a landing pad with a quickstart and links; the CLI reference moved to `cli/README.md`; the rule reference stays in `lints/README.md`.

## [0.1.7] - 2026-05-12

### Added
- `cargo oneway update` subcommand — runs `cargo install cargo-oneway --force --locked`.

### Changed
- **The CLI now pins the lint library to the matching git tag.** Each published `cargo-oneway` embeds its version and passes `--tag vX.Y.Z` to `cargo dylint`. Before this, the lint library was fetched from upstream HEAD, so the rules a user got could change without a CLI update. The dylint cache is keyed per-tag, so CLI bumps automatically invalidate the cache and trigger a rebuild on the next run.
- Release workflow reordered to publish the git tag *before* the crates.io binary, closing a window where users installing the new version would fail to fetch the lint library.

## [0.1.6] - 2026-05-12

### Added
- New lint **`no_self_orchestration`** (deny): a public method in an inherent `impl` must not call another public method on `self`. Private helper methods on `self` are fine. The rule catches the antipattern of a Store / Repository struct accumulating workflow logic via its own public API. Universal — no naming conventions or type denylists needed.

## [0.1.5] - 2026-05-12

### Changed
- **`type_derived_naming` is now generic-aware.** For a binding with a generic type:
  - 0 effective bounds (filtering `Sized`/`Send`/`Sync`/`Unpin`): no constraint.
  - 1 effective bound (`<M: Migrator>`): binding must match the trait name → `migrator: M`.
  - 2+ effective bounds (`<M: A + B + C>`): the generic itself must be given a descriptive role name (not a single uppercase letter); the binding then matches it. So `<Service: A + B + C>(service: Service)` passes.
- **`one_constructor_name` switched from an allowlist to a denylist** of actual near-synonyms (`build`, `construct`, `create`, `init`, `make`). Descriptive constructor names like `from_string`, `with_capacity`, `Message::user`/`system`/`assistant` now pass.

## [0.1.4] - 2026-05-12

### Removed
- **`no_loop` lint** — the ban on `for` (not just `loop`/`while`) was more friction than payoff in practice; replacing `for x in xs { side_effect(x) }` with `xs.iter().for_each(...)` had no real readability gain.
- **`no_turbofish` lint** — legitimate sites like `std::mem::size_of::<T>()` have no good binding to ascribe.

## [0.1.3] - 2026-05-12

### Added
- `justfile` with a `just oneway` recipe that runs `cargo oneway` against both crates in this repo.

### Fixed
- Internal `raw_primitive_field` / `raw_primitive_param` warnings on `bool` / `usize` / `&str` in private helpers — wrapped in newtypes (`Msg`, `BindingName`) or inlined to eliminate the noise.

## [0.1.2] - 2026-05-12

### Changed
- **Self-hosting pass**: the repo now passes its own lint set with zero errors. Touched both `cli/` and `lints/` — bundled multi-arg functions into Opts structs, renamed `cx` → `early_context` (and similar) across all lint passes, converted `for`/`while` loops to iterator chains where reasonable, and added `#[allow(...)]` with `WHY:` rationale on the byte-level state machines that genuinely can't be expressed as combinators.

### Added
- Pre-push hook (`.githooks/pre-push`) that runs `cargo oneway` against both crates before letting a push through. Opt-in via `git config core.hooksPath .githooks`.

## [0.1.1] - 2026-05-12

### Added
- Initial public release. Extracted from `Almaju/oneway` as two crates: `cli/` (the published `cargo-oneway` binary) and `lints/` (the dylint cdylib).
- Auto-publish workflow: every push to `main` bumps the CLI patch version and publishes to crates.io.
- 23 lints — sorting (struct fields, enum variants, match arms, derives, impl methods, mod-after-use), comments policy, no-panic / no-unwrap, type-derived naming, single-constructor name, primitives-wrapped-in-newtypes, no-self-orchestration, and more.

[0.1.14]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.14
[0.1.12]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.12
[0.1.11]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.11
[0.1.10]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.10
[0.1.8]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.8
[0.1.7]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.7
[0.1.6]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.6
[0.1.5]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.5
[0.1.4]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.4
[0.1.3]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.3
[0.1.2]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.2
[0.1.1]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.1
