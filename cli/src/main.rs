use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use serde_json::Value;

const CLIPPY_TOML: &str = include_str!("../templates/clippy.toml");
const RUSTFMT_TOML: &str = include_str!("../templates/rustfmt.toml");

const CLIPPY_DENY: &[&str] = &[
    "clippy::expect_used",
    "clippy::manual_filter_map",
    "clippy::manual_map",
    "clippy::manual_unwrap_or",
    "clippy::needless_return",
    "clippy::panic",
    "clippy::single_match",
    "clippy::todo",
    "clippy::unimplemented",
    "clippy::uninlined_format_args",
    "clippy::unreachable",
    "clippy::unwrap_used",
    "clippy::wildcard_imports",
];

const DYLINT_GIT: &str = "https://github.com/Almaju/oneway-lints";
const DYLINT_PATTERN: &str = "lints";

/// Env var that points `cargo oneway` at a local checkout of `oneway-lints`
/// instead of fetching it from `DYLINT_GIT`. Set this when iterating on the
/// lint rules so changes are picked up without a push/pull round-trip.
const LINTS_PATH_ENV: &str = "ONEWAY_LINTS_PATH";

/// Per-project config file. Lives at the project root next to `Cargo.toml`.
const ONEWAY_TOML: &str = "oneway.toml";

#[derive(Default)]
struct Disabled {
    clippy: Vec<String>,
    dylint: Vec<String>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FixMode {
    Off,
    On,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FmtMode {
    Apply,
    Check,
}

struct LintOpts<'a> {
    disabled: &'a Disabled,
    fix_mode: FixMode,
    passthrough: &'a [String],
}

/// A `one_public_type_per_file` diagnostic distilled to what we need for
/// extraction: source file path and the first line of the type's text (which
/// is enough to locate the type's declaration in the post-fix source).
struct ExtractTarget {
    file: PathBuf,
    first_line: String,
}

// NOTE: extension traits

trait ArgsExt {
    /// Pull `--fix` out of `self`, returning whether it was present. We strip
    /// it because we forward each fixable flag to the underlying tool
    /// ourselves (clippy and dylint both accept `--fix`, but each also needs
    /// `--allow-dirty --allow-staged` to be usable on a working tree).
    fn extract_fix(&mut self) -> FixMode;
}

impl ArgsExt for Vec<String> {
    fn extract_fix(&mut self) -> FixMode {
        match self.iter().position(|a| a == "--fix") {
            None => FixMode::Off,
            Some(pos) => {
                self.remove(pos);
                FixMode::On
            },
        }
    }
}

trait CommandExt {
    fn announce(&self);
    fn run(self) -> io::Result<i32>;
}

impl CommandExt for Command {
    fn announce(&self) {
        let program = self.get_program().to_string_lossy();
        let args: Vec<String> = self
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        eprintln!("$ {} {}", program, args.join(" "));
    }

    fn run(mut self) -> io::Result<i32> {
        self.announce();
        Ok(self.status()?.code().unwrap_or(1))
    }
}

trait BytesExt {
    /// Walk backward over whitespace + `#[...]` blocks starting at `pos`.
    /// Returns the byte offset of the first attribute (or `pos` if none).
    fn extend_backward(&self, pos: usize) -> usize;
}

impl BytesExt for [u8] {
    fn extend_backward(&self, pos: usize) -> usize {
        let mut probe = pos;
        loop {
            let mut q = probe;
            while q > 0 && self[q - 1].is_ascii_whitespace() {
                q -= 1;
            }
            if q == 0 || self[q - 1] != b']' {
                return probe;
            }
            // NOTE: scan backward for matching `[`, then check for `#` in front.
            let mut depth = 1i32;
            let mut r = q - 1;
            while r > 0 && depth > 0 {
                r -= 1;
                match self[r] {
                    b'[' => depth -= 1,
                    b']' => depth += 1,
                    _ => {},
                }
            }
            if depth != 0 || r == 0 || self[r - 1] != b'#' {
                return probe;
            }
            probe = r - 1;
        }
    }
}

trait SrcExt {
    /// Find the byte offset (within `self`) of the `}` that closes the `{`
    /// at `self[0]`. Naive — doesn't account for braces in strings or
    /// comments.
    fn find_matching_brace(&self) -> Option<usize>;
    fn is_mod_decl(&self) -> bool;
    /// Parse `(pub|pub(crate)|pub(super)|...)? (struct|enum) <Name>` from
    /// the first line of an item. Returns
    /// `(visibility_with_trailing_space, name)`.
    fn parse_type_decl(&self) -> Option<(String, String)>;
    fn pascal_to_snake(&self) -> String;
    /// Find the byte offset where the extraction prelude should be inserted:
    /// the start of the first non-attribute, non-comment, non-blank,
    /// non-`mod` line. This keeps existing `mod` declarations before the
    /// new ones and the new `use` statements before any existing `use`
    /// block — preserving the `mod_after_use` invariant.
    fn prelude_insert_position(&self) -> usize;
}

impl SrcExt for str {
    fn find_matching_brace(&self) -> Option<usize> {
        let bytes = self.as_bytes();
        let mut depth = 0i32;
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                },
                _ => {},
            }
            i += 1;
        }
        None
    }

    fn is_mod_decl(&self) -> bool {
        let rest = match self.strip_prefix("pub") {
            None => self,
            Some(after) => match after.chars().next() {
                Some('(') => match after.find(')') {
                    None => return false,
                    Some(close) => after[close + 1..].trim_start(),
                },
                Some(c) if c.is_whitespace() => after.trim_start(),
                _ => self,
            },
        };
        rest.starts_with("mod ") && rest.ends_with(';')
    }

    fn parse_type_decl(&self) -> Option<(String, String)> {
        let s = self.trim_start();
        let (vis, rest) = match s.strip_prefix("pub") {
            None => (String::new(), s),
            Some(after_pub) => match after_pub.chars().next() {
                Some('(') => {
                    let close = after_pub.find(')')?;
                    let vis = format!("pub{} ", &after_pub[..close + 1]);
                    (vis, after_pub[close + 1..].trim_start())
                },
                Some(c) if c.is_whitespace() => ("pub ".to_string(), after_pub.trim_start()),
                _ => return None,
            },
        };
        let rest = rest
            .strip_prefix("struct")
            .or_else(|| rest.strip_prefix("enum"))?;
        let rest = rest.trim_start();
        let end = rest
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        match end {
            0 => None,
            _ => Some((vis, rest[..end].to_string())),
        }
    }

    fn pascal_to_snake(&self) -> String {
        self.chars()
            .enumerate()
            .fold(String::with_capacity(self.len() + 4), |mut out, (i, c)| {
                match c.is_ascii_uppercase() {
                    false => out.push(c),
                    true => {
                        if i > 0 {
                            out.push('_');
                        }
                        out.extend(c.to_lowercase());
                    },
                }
                out
            })
    }

    fn prelude_insert_position(&self) -> usize {
        let mut byte = 0;
        let mut lines = self.lines();
        loop {
            let Some(line) = lines.next() else {
                return self.len();
            };
            let trimmed = line.trim_start();
            let advance = || byte + line.len() + 1;
            if trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed.starts_with("#[")
                || trimmed.starts_with("#![")
                || trimmed.is_mod_decl()
            {
                byte = advance();
                continue;
            }
            return byte;
        }
    }
}

trait PathExt {
    /// Apply extractions to a single source file. Each target is located
    /// in the current (post-fix) source by searching for its first line;
    /// we then expand backward over leading `#[...]` attributes and forward
    /// over the matching brace pair to get the full item, write it to its
    /// own file, splice the parent, and add `pub mod`/`pub use` (or
    /// `pub(crate)`) at the top.
    fn apply_extractions(&self, targets: Vec<ExtractTarget>) -> io::Result<()>;
    /// Compute the destination path for an extracted module given the
    /// parent file. For `lib.rs` / `main.rs` siblings live in the same
    /// directory; for any other parent the extracted file goes inside
    /// `<parent_stem>/`.
    fn destination_for(&self, mod_name: &str) -> PathBuf;
}

impl PathExt for Path {
    fn apply_extractions(&self, targets: Vec<ExtractTarget>) -> io::Result<()> {
        let mut source = fs::read_to_string(self)?;
        let mut mod_lines: Vec<String> = Vec::new();
        let mut use_lines: Vec<String> = Vec::new();
        targets.iter().try_for_each(|target| -> io::Result<()> {
            let Some(decl_start) = source.find(&target.first_line) else {
                eprintln!(
                    "cargo-oneway: extract: couldn't locate `{}` in {}",
                    target.first_line.trim(),
                    self.display()
                );
                return Ok(());
            };
            let (vis, type_name) = match target.first_line.parse_type_decl() {
                None => {
                    eprintln!(
                        "cargo-oneway: extract: couldn't parse type name from `{}`",
                        target.first_line.trim()
                    );
                    return Ok(());
                },
                Some(parsed) => parsed,
            };
            let extract_lo = source.as_bytes().extend_backward(decl_start);
            let Some(brace_open) = source[decl_start..].find('{') else {
                return Ok(());
            };
            let Some(brace_close_rel) = source[decl_start + brace_open..].find_matching_brace()
            else {
                return Ok(());
            };
            let extract_hi = decl_start + brace_open + brace_close_rel + 1;
            let mod_name = type_name.pascal_to_snake();
            let dest = self.destination_for(&mod_name);
            // WHY: don't clobber an unrelated file the user already has.
            // If the destination exists, leave it and skip this extraction;
            // the lint will still fire next run and the human can resolve
            // the conflict.
            if dest.exists() {
                eprintln!(
                    "cargo-oneway: extract: {} already exists, skipping {type_name}",
                    dest.display()
                );
                return Ok(());
            }
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            let extracted = source[extract_lo..extract_hi].trim_start().to_string();
            let extracted = format!("{}\n", extracted.trim_end());
            fs::write(&dest, &extracted)?;
            eprintln!("cargo-oneway: extracted {type_name} → {}", dest.display());
            let prelude_vis = match vis.is_empty() {
                false => vis.trim_end().to_string(),
                true => "pub".to_string(),
            };
            // WHY: emit mods and uses to separate buckets so the final
            // prelude reads `mod a; mod b; \n use a::A; use b::B;`.
            // Interleaving them would violate our own `mod_after_use` rule.
            mod_lines.push(format!("{prelude_vis} mod {mod_name};\n"));
            use_lines.push(format!("{prelude_vis} use {mod_name}::{type_name};\n"));
            // WHY: splice out the extracted bytes along with the trailing
            // blank line that typically separated this item from the next
            // so the parent file doesn't accumulate empty lines on each run.
            let mut splice_hi = extract_hi;
            while source.as_bytes().get(splice_hi) == Some(&b'\n') {
                splice_hi += 1;
                if source.as_bytes().get(splice_hi) != Some(&b'\n') {
                    break;
                }
            }
            source.replace_range(extract_lo..splice_hi, "");
            Ok(())
        })?;
        if mod_lines.is_empty() {
            return Ok(());
        }
        let prelude = format!("{}\n{}\n", mod_lines.join(""), use_lines.join(""));
        let insert_pos = source.prelude_insert_position();
        source.insert_str(insert_pos, &prelude);
        fs::write(self, source)?;
        Ok(())
    }

    fn destination_for(&self, mod_name: &str) -> PathBuf {
        let stem = self.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let dir = self.parent().unwrap_or_else(|| Path::new("."));
        match stem {
            "lib" | "main" => dir.join(format!("{mod_name}.rs")),
            _ => dir.join(stem).join(format!("{mod_name}.rs")),
        }
    }
}

// NOTE: LintOpts methods

impl LintOpts<'_> {
    fn build_dylint_command(&self) -> Command {
        let mut command = Command::new("cargo");
        command.arg("dylint");
        // WHY: either `--path` or `--git --pattern` picks the library. We
        // deliberately do not pass `--lib` because it conflicts with
        // dylint's `--all` mode that gets engaged automatically when
        // `--pattern` is given.
        match env::var(LINTS_PATH_ENV) {
            Ok(path) if !path.is_empty() => {
                command.arg("--path").arg(path);
            },
            _ => {
                // WHY: pin to the git tag matching this CLI's version so
                // the rules a user gets are deterministic from the `cargo
                // install` they ran. Without the tag pin, dylint pulls
                // upstream HEAD and the rules can change without a CLI
                // update.
                command
                    .arg("--git")
                    .arg(DYLINT_GIT)
                    .arg("--tag")
                    .arg(concat!("v", env!("CARGO_PKG_VERSION")))
                    .arg("--pattern")
                    .arg(DYLINT_PATTERN);
            },
        }
        // WHY: per-lint allow-overrides go through RUSTFLAGS — dylint
        // forwards post-`--` args to cargo check, which doesn't pass them
        // to rustc as lint flags. RUSTFLAGS hits rustc directly.
        let parts: Vec<String> = env::var("RUSTFLAGS")
            .ok()
            .filter(|s| !s.is_empty())
            .into_iter()
            .chain(self.disabled.dylint.iter().map(|lint| format!("-A {lint}")))
            .collect();
        if !parts.is_empty() {
            command.env("RUSTFLAGS", parts.join(" "));
        }
        command
    }

    fn collect_extract_targets(&self) -> io::Result<Vec<ExtractTarget>> {
        let mut command = self.build_dylint_command();
        // WHY: `cargo dylint` forwards post-`--` args to the inner cargo
        // invocation, which is where `--message-format` is parsed.
        command.arg("--").arg("--message-format=json");
        command.announce();
        let output = command
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()?;
        let mut targets = Vec::new();
        output.stdout.split(|&b| b == b'\n').for_each(|line| {
            if line.is_empty() {
                return;
            }
            let Ok(value) = serde_json::from_slice::<Value>(line) else {
                return;
            };
            if value.get("reason").and_then(Value::as_str) != Some("compiler-message") {
                return;
            }
            let Some(message) = value.get("message") else {
                return;
            };
            let code = message
                .get("code")
                .and_then(|c| c.get("code"))
                .and_then(Value::as_str);
            if code != Some("one_public_type_per_file") {
                return;
            }
            let Some(spans) = message.get("spans").and_then(Value::as_array) else {
                return;
            };
            let Some(primary) = spans
                .iter()
                .find(|s| s.get("is_primary").and_then(Value::as_bool) == Some(true))
            else {
                return;
            };
            let file = primary.get("file_name").and_then(Value::as_str);
            let text = primary
                .get("text")
                .and_then(Value::as_array)
                .and_then(|arr| arr.first())
                .and_then(|t| t.get("text"))
                .and_then(Value::as_str);
            if let (Some(file), Some(text)) = (file, text) {
                targets.push(ExtractTarget {
                    file: PathBuf::from(file),
                    first_line: text.to_string(),
                });
            }
        });
        Ok(targets)
    }

    fn run_all(&self) -> io::Result<i32> {
        let fmt_mode = match self.fix_mode {
            FixMode::Off => FmtMode::Check,
            FixMode::On => FmtMode::Apply,
        };
        let fmt = self.run_fmt(fmt_mode)?;
        let clippy = self.run_clippy()?;
        let dylint = self.run_dylint()?;
        Ok([fmt, clippy, dylint]
            .into_iter()
            .find(|&c| c != 0)
            .unwrap_or(0))
    }

    fn run_clippy(&self) -> io::Result<i32> {
        let dir = write_config_dir()?;
        let mut command = Command::new("cargo");
        command.arg("clippy");
        match self.fix_mode {
            FixMode::Off => {},
            FixMode::On => {
                command
                    .arg("--fix")
                    .arg("--allow-dirty")
                    .arg("--allow-staged");
            },
        }
        command.args(self.passthrough);
        command.arg("--");
        CLIPPY_DENY.iter().for_each(|lint| {
            command.arg("-D").arg(lint);
        });
        // WHY: allow-overrides come after the deny defaults so per-project opt-outs win.
        self.disabled.clippy.iter().for_each(|lint| {
            command.arg("-A").arg(lint);
        });
        command.env("CLIPPY_CONF_DIR", &dir);
        command.run()
    }

    fn run_dylint(&self) -> io::Result<i32> {
        let mut command = self.build_dylint_command();
        match self.fix_mode {
            FixMode::Off => {},
            FixMode::On => {
                command.arg("--fix");
                // WHY: --broken-code is required so cargo fix doesn't
                // revert suggestions whose intermediate output doesn't
                // typecheck. Many of our autofixes are deliberately
                // starting points (newtype wrappers leave call sites
                // broken, etc.) — without this flag those edits get
                // rolled back and the user sees no change.
                command
                    .arg("--")
                    .arg("--allow-dirty")
                    .arg("--allow-staged")
                    .arg("--broken-code");
            },
        }
        let exit = command.run()?;
        match self.fix_mode {
            FixMode::Off => Ok(exit),
            // WHY: run an extraction pass after the rustc-suggestion-based
            // fix. `one_public_type_per_file` has no in-source suggestion
            // (it needs to create new files), so we collect those
            // diagnostics from a second dylint invocation with
            // `--message-format=json` and apply the splits ourselves.
            // Extract failures don't fail the lint command — the
            // diagnostic still got reported.
            FixMode::On => {
                let _ = self.run_extract_pass();
                Ok(exit)
            },
        }
    }

    fn run_extract_pass(&self) -> io::Result<()> {
        let targets = self.collect_extract_targets()?;
        // WHY: group by file and apply extractions back-to-front so
        // earlier byte offsets in the same file stay valid after each
        // splice.
        let mut by_file: HashMap<PathBuf, Vec<ExtractTarget>> = HashMap::new();
        targets.into_iter().for_each(|t| {
            by_file.entry(t.file.clone()).or_default().push(t);
        });
        by_file.into_iter().for_each(|(file, targets)| {
            if let Err(error) = file.apply_extractions(targets) {
                eprintln!("cargo-oneway: extract pass on {}: {error}", file.display());
            }
        });
        Ok(())
    }

    fn run_fmt(&self, fmt_mode: FmtMode) -> io::Result<i32> {
        let dir = write_config_dir()?;
        let mut command = Command::new("cargo");
        command.arg("fmt");
        command.args(self.passthrough);
        match fmt_mode {
            FmtMode::Apply => {},
            FmtMode::Check => {
                command.arg("--check");
            },
        }
        command
            .arg("--")
            .arg("--config-path")
            .arg(dir.join("rustfmt.toml"));
        command.run()
    }

    fn run_lint(&self) -> io::Result<i32> {
        let clippy = self.run_clippy()?;
        let dylint = self.run_dylint()?;
        Ok([clippy, dylint].into_iter().find(|&c| c != 0).unwrap_or(0))
    }
}

// NOTE: zero-arg free fns (entry points)

/// Read `oneway.toml` from the current directory and partition
/// `disable = [...]` entries into clippy-prefixed names and bare dylint lint
/// names.
fn read_disabled() -> Disabled {
    let Ok(content) = fs::read_to_string(ONEWAY_TOML) else {
        return Disabled::default();
    };
    let value: toml::Value = match content.parse() {
        Err(e) => {
            eprintln!("cargo-oneway: {ONEWAY_TOML}: {e}");
            return Disabled::default();
        },
        Ok(v) => v,
    };
    let Some(array) = value.get("disable").and_then(toml::Value::as_array) else {
        return Disabled::default();
    };
    array.iter().filter_map(|entry| entry.as_str()).fold(
        Disabled::default(),
        |mut disabled, name| {
            match name.strip_prefix("clippy::") {
                None => disabled.dylint.push(name.to_string()),
                Some("") => {},
                Some(_) => disabled.clippy.push(name.to_string()),
            }
            disabled
        },
    )
}

fn user_args() -> Vec<String> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("oneway") {
        args.remove(0);
    }
    args
}

fn write_config_dir() -> io::Result<PathBuf> {
    let dir = env::temp_dir().join(format!("cargo-oneway-{}", std::process::id()));
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("clippy.toml"), CLIPPY_TOML)?;
    fs::write(dir.join("rustfmt.toml"), RUSTFMT_TOML)?;
    Ok(dir)
}

fn run_update() -> io::Result<i32> {
    let mut command = Command::new("cargo");
    command.args(["install", "cargo-oneway", "--force", "--locked"]);
    command.run()
}

fn print_help() {
    eprintln!(
        "cargo-oneway — opinionated lint + format runner

USAGE:
    cargo oneway [SUBCOMMAND] [--fix] [CARGO_ARGS...]

SUBCOMMANDS:
    fmt        Apply Oneway rustfmt config to the workspace
    lint       Run clippy + oneway-lints with the Oneway lint set
    update     Reinstall the latest `cargo-oneway` from crates.io
    version    Print the installed CLI version (also: --version, -V)
    help       Print this message

With no subcommand, runs `fmt --check`, clippy, and oneway-lints — failing
if any step fails. CARGO_ARGS are forwarded to the underlying cargo command.

FLAGS:
    --fix   Apply autofixes: rewrites formatting in place (no `--check`),
            and runs clippy + oneway-lints with `--fix --allow-dirty
            --allow-staged` so they can patch a dirty working tree.
            After the rustc-suggestion-based fixes, an extraction pass
            handles `one_public_type_per_file` by moving extra primary
            public types to their own files and rewiring `mod`/`use`.

CONFIG:
    oneway.toml at the project root can disable specific rules:
        disable = [\"type_derived_naming\", \"clippy::wildcard_imports\"]

ENVIRONMENT:
    ONEWAY_LINTS_PATH   Path to a local `oneway-lints` checkout. When set,
                        dylint builds from that path instead of cloning the
                        upstream git repo. Use this when iterating on the
                        lint rules.

PREREQUISITES:
    cargo install cargo-dylint dylint-link
"
    );
}

fn dispatch() -> io::Result<i32> {
    let mut args = user_args();
    let fix_mode = args.extract_fix();
    let disabled = read_disabled();
    let subcommand = args.first().map(String::as_str);
    let passthrough = args.get(1..).unwrap_or(&[]);
    let lint_opts = LintOpts {
        disabled: &disabled,
        fix_mode,
        passthrough,
    };
    match subcommand {
        None => lint_opts.run_all(),
        Some("--help") | Some("-h") | Some("help") => {
            print_help();
            Ok(0)
        },
        Some("--version") | Some("-V") | Some("version") => {
            println!("cargo-oneway {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        },
        Some("fmt") => lint_opts.run_fmt(FmtMode::Apply),
        Some("lint") => lint_opts.run_lint(),
        Some("update") => run_update(),
        Some(other) => {
            eprintln!("cargo-oneway: unknown subcommand `{other}` — try `cargo oneway help`");
            Ok(2)
        },
    }
}

fn main() -> ExitCode {
    match dispatch() {
        Err(e) => {
            eprintln!("cargo-oneway: {e}");
            ExitCode::FAILURE
        },
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => ExitCode::from(code.clamp(1, 255) as u8),
    }
}
