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

## Rules

Click any rule for its full rationale and bad/good examples.

| Rule | Severity | Summary |
|------|----------|---------|
| [`inline_format_args`](lints/docs/inline_format_args.md) | deny | Use inline capture in format strings (`{name}`, not `"{}", name`). |
| [`mod_after_use`](lints/docs/mod_after_use.md) | deny | `mod` declarations must appear before any `use`. |
| [`no_comments`](lints/docs/no_comments.md) | deny | Non-doc comments must carry a `WHY:`/`SAFETY:`/`TODO:`/… label, a link, or a ticket ref. |
| [`no_explicit_return`](lints/docs/no_explicit_return.md) | warn | No `return` keyword when a trailing expression works. |
| [`no_glob_imports`](lints/docs/no_glob_imports.md) | deny | No wildcard imports — every symbol must be named. |
| [`no_if_else`](lints/docs/no_if_else.md) | warn | Prefer `match` over `if`/`else` chains. |
| [`no_nested_functions`](lints/docs/no_nested_functions.md) | warn | No functions defined inside other functions. |
| [`no_panic`](lints/docs/no_panic.md) | deny | No `panic!`/`todo!`/`unimplemented!`/`unreachable!` in non-test code. |
| [`no_self_orchestration`](lints/docs/no_self_orchestration.md) | deny | A pub method must not call another pub method on `self` — extract workflows. |
| [`no_unwrap`](lints/docs/no_unwrap.md) | deny | No `.unwrap()`/`.expect()` in non-test code. |
| [`one_constructor_name`](lints/docs/one_constructor_name.md) | deny | Constructors must be named `new` (not `create`/`build`/`init`/…). |
| [`one_public_type_per_file`](lints/docs/one_public_type_per_file.md) | warn | At most one primary public type per file. |
| [`prefer_combinators`](lints/docs/prefer_combinators.md) | warn | Use `Option`/`Result` combinators over `match` for simple transforms. |
| [`raw_primitive_field`](lints/docs/raw_primitive_field.md) | warn | Struct fields should use newtypes, not raw primitives. |
| [`raw_primitive_param`](lints/docs/raw_primitive_param.md) | warn | Function parameters should use newtypes, not raw primitives. |
| [`too_many_params`](lints/docs/too_many_params.md) | deny | Functions take at most 2 parameters (including `&self`). |
| [`type_derived_naming`](lints/docs/type_derived_naming.md) | deny | Binding name must be the `snake_case` of its type. |
| [`unsorted_derives`](lints/docs/unsorted_derives.md) | deny | `#[derive(...)]` traits must be alphabetical. |
| [`unsorted_enum_variants`](lints/docs/unsorted_enum_variants.md) | deny | Enum variants must be alphabetical. |
| [`unsorted_impl_methods`](lints/docs/unsorted_impl_methods.md) | deny | `impl` methods: constructors, then public, then private — alphabetical within each. |
| [`unsorted_match_arms`](lints/docs/unsorted_match_arms.md) | deny | Match arms must be sorted by pattern; `_` last. |
| [`unsorted_struct_fields`](lints/docs/unsorted_struct_fields.md) | deny | Struct fields must be alphabetical. |

## Opt-out per project: `oneway.toml`

Drop a `oneway.toml` at your project root to disable specific rules:

```toml
disable = [
    "type_derived_naming",          # a dylint rule from this crate
    "clippy::wildcard_imports",     # a clippy rule
]
```

Names without a prefix target the dylint library; names prefixed with `clippy::` target clippy.

## Repository Layout

| Path | Description |
|------|-------------|
| [`lints/`](lints/) | The dylint cdylib (`oneway_lints`). Pinned to a specific nightly. |
| [`cli/`](cli/) | The `cargo-oneway` binary, published to crates.io. |

## Contributing

A `pre-push` hook in [`.githooks/`](.githooks/) runs `cargo oneway` against both crates before letting a push through. Enable it once per clone:

```sh
git config core.hooksPath .githooks
```

The hook uses your local `lints/` checkout (via `ONEWAY_LINTS_PATH`), so it lints against the rules you're actually committing. Bypass with `git push --no-verify` if you need to.

## Releases

`cargo-oneway` ships on every push to `main`: the [release workflow](.github/workflows/release.yml) bumps the patch version, publishes to crates.io, and commits the bump back as `chore: release cargo-oneway vX.Y.Z`. No manual tagging required. Version inflation is the cost; reproducibility (pinned versions in users' `Cargo.lock`) is the benefit.

The dylint library (`lints/`) is consumed via git (`cargo dylint --git ...`), so it doesn't have a release cadence — every push to `main` is immediately picked up by the next `cargo oneway` invocation that hits the dylint cache miss.

## License

Dual-licensed under MIT or Apache-2.0.
