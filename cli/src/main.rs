use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

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
    "clippy::too_many_arguments",
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

/// Read `oneway.toml` from the current directory and partition `disable = [...]`
/// entries into clippy-prefixed names and bare dylint lint names.
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

/// Pull `--fix` out of `args`, returning whether it was present. We strip it
/// because we forward each fixable flag to the underlying tool ourselves
/// (clippy and dylint both accept `--fix`, but each also needs
/// `--allow-dirty --allow-staged` to be usable on a working tree).
fn extract_fix(args: &mut Vec<String>) -> FixMode {
    match args.iter().position(|a| a == "--fix") {
        None => FixMode::Off,
        Some(pos) => {
            args.remove(pos);
            FixMode::On
        },
    }
}

fn write_config_dir() -> io::Result<PathBuf> {
    let dir = env::temp_dir().join(format!("cargo-oneway-{}", std::process::id()));
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("clippy.toml"), CLIPPY_TOML)?;
    fs::write(dir.join("rustfmt.toml"), RUSTFMT_TOML)?;
    Ok(dir)
}

fn announce(command: &Command) {
    let program = command.get_program().to_string_lossy();
    let args: Vec<String> = command
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    eprintln!("$ {} {}", program, args.join(" "));
}

fn run(mut command: Command) -> io::Result<i32> {
    announce(&command);
    Ok(command.status()?.code().unwrap_or(1))
}

fn run_fmt(passthrough: &[String], fmt_mode: FmtMode) -> io::Result<i32> {
    let dir = write_config_dir()?;
    let mut command = Command::new("cargo");
    command.arg("fmt");
    command.args(passthrough);
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
    run(command)
}

fn run_clippy(lint_opts: &LintOpts<'_>) -> io::Result<i32> {
    let dir = write_config_dir()?;
    let mut command = Command::new("cargo");
    command.arg("clippy");
    match lint_opts.fix_mode {
        FixMode::Off => {},
        FixMode::On => {
            command
                .arg("--fix")
                .arg("--allow-dirty")
                .arg("--allow-staged");
        },
    }
    command.args(lint_opts.passthrough);
    command.arg("--");
    CLIPPY_DENY.iter().for_each(|lint| {
        command.arg("-D").arg(lint);
    });
    // WHY: allow-overrides come after the deny defaults so per-project opt-outs win.
    lint_opts.disabled.clippy.iter().for_each(|lint| {
        command.arg("-A").arg(lint);
    });
    command.env("CLIPPY_CONF_DIR", &dir);
    run(command)
}

fn run_dylint(lint_opts: &LintOpts<'_>) -> io::Result<i32> {
    let mut command = Command::new("cargo");
    command.arg("dylint");
    match lint_opts.fix_mode {
        FixMode::Off => {},
        FixMode::On => {
            command.arg("--fix");
        },
    }
    // WHY: either `--path` or `--git --pattern` picks the library. We deliberately
    // do not pass `--lib` because it conflicts with dylint's `--all` mode that
    // gets engaged automatically when `--pattern` is given.
    match env::var(LINTS_PATH_ENV) {
        Ok(path) if !path.is_empty() => {
            command.arg("--path").arg(path);
        },
        _ => {
            command
                .arg("--git")
                .arg(DYLINT_GIT)
                .arg("--pattern")
                .arg(DYLINT_PATTERN);
        },
    }
    match lint_opts.fix_mode {
        FixMode::Off => {},
        FixMode::On => {
            command.arg("--").arg("--allow-dirty").arg("--allow-staged");
        },
    }
    // WHY: per-lint allow-overrides go through RUSTFLAGS — dylint forwards
    // post-`--` args to cargo check, which doesn't pass them to rustc as
    // lint flags. RUSTFLAGS hits rustc directly.
    let parts: Vec<String> = env::var("RUSTFLAGS")
        .ok()
        .filter(|s| !s.is_empty())
        .into_iter()
        .chain(
            lint_opts
                .disabled
                .dylint
                .iter()
                .map(|lint| format!("-A {lint}")),
        )
        .collect();
    if !parts.is_empty() {
        command.env("RUSTFLAGS", parts.join(" "));
    }
    run(command)
}

fn run_lint(lint_opts: &LintOpts<'_>) -> io::Result<i32> {
    let clippy = run_clippy(lint_opts)?;
    let dylint = run_dylint(lint_opts)?;
    Ok([clippy, dylint].into_iter().find(|&c| c != 0).unwrap_or(0))
}

fn run_all(lint_opts: &LintOpts<'_>) -> io::Result<i32> {
    let fmt_mode = match lint_opts.fix_mode {
        FixMode::Off => FmtMode::Check,
        FixMode::On => FmtMode::Apply,
    };
    let fmt = run_fmt(lint_opts.passthrough, fmt_mode)?;
    let clippy = run_clippy(lint_opts)?;
    let dylint = run_dylint(lint_opts)?;
    Ok([fmt, clippy, dylint]
        .into_iter()
        .find(|&c| c != 0)
        .unwrap_or(0))
}

fn print_help() {
    eprintln!(
        "cargo-oneway — opinionated lint + format runner

USAGE:
    cargo oneway [SUBCOMMAND] [--fix] [CARGO_ARGS...]

SUBCOMMANDS:
    fmt     Apply Oneway rustfmt config to the workspace
    lint    Run clippy + oneway-lints with the Oneway lint set
    help    Print this message

With no subcommand, runs `fmt --check`, clippy, and oneway-lints — failing
if any step fails. CARGO_ARGS are forwarded to the underlying cargo command.

FLAGS:
    --fix   Apply autofixes: rewrites formatting in place (no `--check`),
            and runs clippy + oneway-lints with `--fix --allow-dirty
            --allow-staged` so they can patch a dirty working tree.

CONFIG:
    oneway.toml at the project root can disable specific rules:
        disable = [\"no_loop\", \"clippy::wildcard_imports\"]

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
    let fix_mode = extract_fix(&mut args);
    let disabled = read_disabled();
    let subcommand = args.first().map(String::as_str);
    let passthrough = args.get(1..).unwrap_or(&[]);
    let lint_opts = LintOpts {
        disabled: &disabled,
        fix_mode,
        passthrough,
    };
    match subcommand {
        None => run_all(&lint_opts),
        Some("--help") | Some("-h") | Some("help") => {
            print_help();
            Ok(0)
        },
        Some("fmt") => run_fmt(passthrough, FmtMode::Apply),
        Some("lint") => run_lint(&lint_opts),
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
