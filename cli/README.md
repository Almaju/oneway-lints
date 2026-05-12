# cargo-oneway

One command that runs `rustfmt`, `clippy`, and `oneway-lints` (via dylint) against your Rust project with the Oneway lint set baked in. No copy-paste of config files into every project.

## Install

```sh
cargo install cargo-dylint dylint-link
cargo install cargo-oneway
```

## Use

```sh
cargo oneway          # fmt --check + clippy + oneway-lints
cargo oneway fmt      # apply formatting
cargo oneway lint     # clippy + oneway-lints only
cargo oneway --fix    # autofix what's autofixable
cargo oneway help
```

The first `cargo oneway` invocation triggers `cargo dylint` to clone the `oneway-lints` library from the upstream repo and build it under the pinned nightly. Subsequent runs use the dylint cache.

## Opt-out per project: `oneway.toml`

Drop a `oneway.toml` at the project root to silence specific rules for that project:

```toml
disable = [
    "type_derived_naming",      # a dylint rule (from oneway-lints)
    "clippy::wildcard_imports", # a clippy rule
]
```

The CLI routes entries by prefix: `clippy::*` are appended as `-A clippy::<name>` to the clippy invocation; everything else goes to dylint via `RUSTFLAGS=-A <name>`.

## What it does

| Step | Tool | Config source |
|------|------|---------------|
| Format | `cargo fmt` | [`templates/rustfmt.toml`](templates/rustfmt.toml) |
| General lints | `cargo clippy` | [`templates/clippy.toml`](templates/clippy.toml) + a curated deny-list (see [`src/main.rs`](src/main.rs)) |
| Oneway-specific lints | `cargo dylint --lib oneway_lints` | From the [`lints/`](../lints/) sibling crate |

## Why these clippy lints?

The deny-list in `src/main.rs` covers the Oneway lint suite's principles that clippy already handles natively — no point reimplementing what clippy does for free. See [`../lints/README.md`](../lints/README.md) for the full picture.
