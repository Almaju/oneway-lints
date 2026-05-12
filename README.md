# oneway-lints

An opinionated lint suite for Rust. One command — `cargo oneway` — runs `rustfmt`, `clippy` (with the Oneway config), and a custom [dylint](https://github.com/trailofbits/dylint) library against your project. No copy-paste of config files into every repo.

The philosophy: **there is one way to do it**. Sorting is canonical, comments must justify themselves, raw primitives are wrapped, builders are out (struct literals in), and so on. See [`lints/docs/`](lints/docs/) for every rule, with rationale and bad/good examples.

## Install

```sh
cargo install cargo-dylint dylint-link
cargo install cargo-oneway
```

## Use

```sh
cargo oneway          # check formatting + clippy + oneway-lints
cargo oneway fmt      # apply formatting
cargo oneway lint     # lint only
cargo oneway help
```

The first `cargo oneway` invocation triggers `cargo dylint` to clone the lint library from this repo and build it under the pinned nightly. Subsequent runs use the dylint cache (`~/.dylint`).

## Opt-out per project: `oneway.toml`

Drop a `oneway.toml` at your project root to disable specific rules:

```toml
disable = [
    "no_loop",                # a dylint rule from this crate
    "type_derived_naming",
    "clippy::wildcard_imports", # a clippy rule
]
```

Names without a prefix target the dylint library; names prefixed with `clippy::` target clippy.

## Repository Layout

| Path | Description |
|------|-------------|
| [`lints/`](lints/) | The dylint cdylib (`oneway_lints`). Pinned to a specific nightly. |
| [`cli/`](cli/) | The `cargo-oneway` binary, published to crates.io. |

## License

Dual-licensed under MIT or Apache-2.0.
