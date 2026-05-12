# Changelog

All notable changes to the `cargo-oneway` CLI and the bundled `oneway-lints` library.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). From v0.1.7 onward, the CLI version pins the lint library to the matching `vX.Y.Z` git tag — `cargo install cargo-oneway --version X.Y.Z` gives you exactly the rules from this section.

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

[0.1.8]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.8
[0.1.7]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.7
[0.1.6]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.6
[0.1.5]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.5
[0.1.4]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.4
[0.1.3]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.3
[0.1.2]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.2
[0.1.1]: https://github.com/Almaju/oneway-lints/releases/tag/v0.1.1
