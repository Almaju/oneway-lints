# cargo-oneway

The `cargo oneway` CLI. Runs `rustfmt`, `clippy`, and `oneway-lints` (via dylint) against your Rust project with the Oneway lint set baked in — no copy-paste of config files into every project.

For *what rules are enforced*, see [`../lints/README.md`](../lints/README.md). This file documents *how to drive the CLI*.

## Install

```sh
cargo install cargo-dylint dylint-link
cargo install cargo-oneway
```

`cargo-dylint` and `dylint-link` are one-time installs; `cargo-oneway` is what updates as the rules evolve.

## Subcommands

| Command | Description |
|---------|-------------|
| `cargo oneway` | `fmt --check` + `clippy` + `oneway-lints`. Fails if any step fails. |
| `cargo oneway fmt` | Apply rustfmt (with the Oneway config) — rewrites files in place. |
| `cargo oneway lint` | Run `clippy` + `oneway-lints` only (skip formatting). |
| `cargo oneway update` | Reinstall the latest `cargo-oneway` from crates.io. |
| `cargo oneway version` | Print the installed CLI version. Also responds to `--version` / `-V`. |
| `cargo oneway help` | Print usage. Also `--help` / `-h`. |

## Flags

- `--fix` — apply autofixes: rewrites formatting in place (no `--check`) and runs `clippy` + `oneway-lints` with `--fix --allow-dirty --allow-staged` so they can patch a dirty working tree.

Any other arguments are forwarded to the underlying cargo command (e.g. `cargo oneway lint --package foo`).

## Configuration: `oneway.toml`

Drop a `oneway.toml` at the project root to silence specific rules for that project:

```toml
disable = [
    "type_derived_naming",      # a dylint rule (from oneway-lints)
    "clippy::wildcard_imports", # a clippy rule
]
```

The CLI routes entries by prefix: `clippy::*` entries are appended as `-A clippy::<name>` to the clippy invocation; everything else goes to dylint via `RUSTFLAGS=-A <name>`.

## Environment

| Variable | Purpose |
|----------|---------|
| `ONEWAY_LINTS_PATH` | Path to a local `oneway-lints` checkout. When set, dylint builds from that path instead of fetching the upstream git tag. Use this when iterating on the lint rules themselves. |
| `RUSTFLAGS` | Existing flags are preserved; per-project dylint allow-overrides are appended. |

## Version coupling

Each published `cargo-oneway` binary embeds its own version and tells `cargo dylint` to fetch the lints from the matching git tag (`--tag vX.Y.Z`). So `cargo install cargo-oneway --version 0.1.5` gives you exactly the rules that shipped with v0.1.5 — not whatever happens to be on `main` today.

Updating the rules is one command:

```sh
cargo oneway update
```

The dylint cache is keyed per-tag, so the CLI bump naturally invalidates the cache and the next `cargo oneway` invocation rebuilds the lint library at the new tag.

## What it does internally

| Step | Tool | Config source |
|------|------|---------------|
| Format | `cargo fmt` | [`templates/rustfmt.toml`](templates/rustfmt.toml) |
| General lints | `cargo clippy` | [`templates/clippy.toml`](templates/clippy.toml) + a curated deny-list (see [`src/main.rs`](src/main.rs)) |
| Oneway-specific lints | `cargo dylint --git ... --tag v<version> --pattern lints` | The [`lints/`](../lints/) sibling crate, fetched at the matching tag |

The clippy deny-list covers the Oneway principles that clippy already implements natively — no point reimplementing what clippy does for free. See [`../lints/README.md`](../lints/README.md) for the full rule set and which tool enforces each.
