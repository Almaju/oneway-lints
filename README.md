# oneway-lints

An opinionated lint suite for Rust. One command — `cargo oneway` — runs `rustfmt`, `clippy` (with the Oneway config), and a custom [dylint](https://github.com/trailofbits/dylint) library against your project. No copy-paste of config files into every repo.

The philosophy: **there is one way to do it**. Sorting is canonical, comments must justify themselves, raw primitives are wrapped, public methods don't compose other public methods, and so on.

## Quickstart

```sh
cargo install cargo-dylint dylint-link
cargo install cargo-oneway
cargo oneway
```

The first `cargo oneway` invocation triggers `cargo dylint` to clone the lint library at the matching git tag and build it under the pinned nightly. Subsequent runs use the dylint cache.

## Documentation

- [**`cli/README.md`**](cli/README.md) — how to use the CLI: every subcommand, flags, `oneway.toml`, environment, version coupling.
- [**`lints/README.md`**](lints/README.md) — every rule, grouped by category, with severities and links to per-rule rationale + examples.
- [**`lints/docs/`**](lints/docs/) — one Markdown page per rule with bad/good examples.

## Repository Layout

| Path | Description |
|------|-------------|
| [`cli/`](cli/) | The `cargo-oneway` binary, published to crates.io. |
| [`lints/`](lints/) | The dylint cdylib (`oneway_lints`). Pinned to a specific nightly. |

## Contributing

A `pre-push` hook in [`.githooks/`](.githooks/) runs `cargo oneway` against both crates before letting a push through. Enable it once per clone:

```sh
git config core.hooksPath .githooks
```

The hook uses your local `lints/` checkout (via `ONEWAY_LINTS_PATH`), so it lints against the rules you're actually committing. Bypass with `git push --no-verify` if you need to.

There's also a `justfile` recipe: `just oneway` runs the same checks across both crates.

## Releases

`cargo-oneway` ships on every push to `main`: the [release workflow](.github/workflows/release.yml) bumps the patch version, commits and tags the bump, pushes the `vX.Y.Z` tag, then publishes to crates.io.

**The CLI version pins the lint library.** Each published `cargo-oneway` binary embeds its own version and asks `cargo dylint` for the matching git tag (`--tag vX.Y.Z`). So `cargo install cargo-oneway --version 0.1.5` gives you exactly the rules that shipped with v0.1.5 — not whatever happens to be on `main` today. Updating the rules means updating the CLI: `cargo oneway update`.

## License

Dual-licensed under MIT or Apache-2.0.
