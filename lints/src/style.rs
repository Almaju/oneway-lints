use rustc_ast::ast;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::{BytePos, FileName, Span};

declare_lint! {
    /// **Deny** — non-doc comments must declare *why* they exist. Allowed:
    /// doc comments (`///`, `//!`, `/** */`, `/*! */`), comments starting
    /// with a label (`WHY:`, `SAFETY:`, `NOTE:`, `HACK:`, `TODO:`, `FIXME:`,
    /// `PERF:`), and comments containing a link (`http://`, `https://`) or
    /// ticket reference (`#1234`). Plain narrating comments like
    /// `// increment by 1` are forbidden — rename or extract instead.
    pub NO_COMMENTS,
    Deny,
    "non-doc comments must declare their purpose with a label, link, or ticket ref"
}

fn is_local_path(path: &std::path::Path) -> bool {
    let s = path.to_string_lossy();
    !s.contains("/.cargo/")
        && !s.contains("/.rustup/")
        && !s.contains("/rustlib/")
        && !s.starts_with("<")
}

// WHY: hand-rolled byte-level lexer that scans Rust source for comments while
// correctly skipping string, raw-string, byte-string, and char literals (and
// distinguishing char literals from lifetimes). This is a state machine; the
// `while`/`if`-`else if` shape is the clearest expression of "advance the
// cursor, dispatch on the current byte, continue." Iterator combinators would
// require a custom Iterator wrapper that re-implements the same state, with
// no readability gain.
#[allow(no_if_else, raw_primitive_param)]
fn find_comments(src: &str) -> Vec<(usize, usize)> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut out = Vec::new();

    while i < len {
        let b = bytes[i];

        if b == b'"' {
            i += 1;
            while i < len {
                match bytes[i] {
                    b'"' => {
                        i += 1;
                        break;
                    },
                    b'\\' if i + 1 < len => i += 2,
                    _ => i += 1,
                }
            }
            continue;
        }

        // NOTE: raw string forms — r"..." / r#"..."# / br"..." / br#"..."#
        let raw_start = match (b, bytes.get(i + 1).copied()) {
            (b'b', Some(b'r')) if bytes.get(i + 2).is_some_and(|&c| c == b'"' || c == b'#') => {
                Some(i + 2)
            },
            (b'r', Some(c)) if c == b'"' || c == b'#' => Some(i + 1),
            _ => None,
        };
        if let Some(after_prefix) = raw_start {
            let mut j = after_prefix;
            let mut hashes = 0;
            while j < len && bytes[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < len && bytes[j] == b'"' {
                i = j + 1;
                while i < len {
                    if bytes[i] == b'"' {
                        let mut k = i + 1;
                        let mut close = 0;
                        while k < len && close < hashes && bytes[k] == b'#' {
                            close += 1;
                            k += 1;
                        }
                        if close == hashes {
                            i = k;
                            break;
                        }
                    }
                    i += 1;
                }
                continue;
            }
        }

        // WHY: a leading `'` could be a char literal or a lifetime. Scan
        // forward for a closing `'` before any non-identifier char: if found,
        // char literal; otherwise, lifetime (skip one byte).
        if b == b'\'' {
            let mut k = i + 1;
            let mut probe = 0;
            let mut found_close = false;
            while k < len && probe < 6 {
                if bytes[k] == b'\\' && k + 1 < len {
                    k += 2;
                    probe += 1;
                    continue;
                }
                if bytes[k] == b'\'' {
                    found_close = true;
                    break;
                }
                if !(bytes[k].is_ascii_alphanumeric() || bytes[k] == b'_') {
                    break;
                }
                k += 1;
                probe += 1;
            }
            if found_close {
                i = k + 1;
            } else {
                i += 1;
            }
            continue;
        }

        if b == b'/' && i + 1 < len {
            match bytes[i + 1] {
                b'*' => {
                    let start = i;
                    let third = bytes.get(i + 2).copied();
                    let fourth = bytes.get(i + 3).copied();
                    let is_outer_doc =
                        third == Some(b'*') && fourth != Some(b'*') && fourth != Some(b'/');
                    let is_inner_doc = third == Some(b'!');
                    let is_doc = is_outer_doc || is_inner_doc;
                    i += 2;
                    let mut depth: u32 = 1;
                    while i + 1 < len && depth > 0 {
                        if bytes[i] == b'/' && bytes[i + 1] == b'*' {
                            depth += 1;
                            i += 2;
                        } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                            depth -= 1;
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    if !is_doc {
                        out.push((start, i));
                    }
                    continue;
                },
                b'/' => {
                    let start = i;
                    let third = bytes.get(i + 2).copied();
                    let fourth = bytes.get(i + 3).copied();
                    let is_outer_doc = third == Some(b'/') && fourth != Some(b'/');
                    let is_inner_doc = third == Some(b'!');
                    let is_doc = is_outer_doc || is_inner_doc;
                    while i < len && bytes[i] != b'\n' {
                        i += 1;
                    }
                    if !is_doc {
                        out.push((start, i));
                    }
                    continue;
                },
                _ => {},
            }
        }

        i += 1;
    }

    out
}

const ALLOWED_LABELS: &[&str] = &["FIXME", "HACK", "NOTE", "PERF", "SAFETY", "TODO", "WHY"];

/// Returns true if the given line of comment content carries a label, link,
/// or ticket reference that justifies the comment's existence.
#[allow(raw_primitive_param)]
fn line_is_justified(line: &str) -> bool {
    let trimmed = line.trim_start_matches(['/', '*']).trim_start();

    let has_label = ALLOWED_LABELS.iter().any(|label| {
        trimmed
            .strip_prefix(label)
            .is_some_and(|rest| rest.starts_with(':'))
    });
    if has_label {
        return true;
    }

    if trimmed.contains("http://") || trimmed.contains("https://") {
        return true;
    }

    let bytes = trimmed.as_bytes();
    bytes
        .iter()
        .enumerate()
        .any(|(i, &b)| b == b'#' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit))
}

#[allow(raw_primitive_param)]
fn is_block_comment(src: &str, lo: usize) -> bool {
    src.as_bytes().get(lo..lo + 2) == Some(b"/*")
}

/// Group consecutive `//` line comments into logical comment blocks.
/// Block comments (`/* */`) are always their own group.
#[allow(raw_primitive_param)]
fn group_comments(src: &str, comments: &[(usize, usize)]) -> Vec<Vec<(usize, usize)>> {
    let bytes = src.as_bytes();
    let mut groups: Vec<Vec<(usize, usize)>> = Vec::new();
    comments.iter().for_each(|&range| {
        let (lo, _) = range;
        let block = is_block_comment(src, lo);
        let extend_into = match block {
            false => groups.last_mut().filter(|g| {
                g.last().is_some_and(|&(prev_lo, prev_hi)| {
                    if is_block_comment(src, prev_lo) {
                        return false;
                    }
                    if prev_hi >= lo || bytes.get(prev_hi) != Some(&b'\n') {
                        return false;
                    }
                    bytes[prev_hi + 1..lo]
                        .iter()
                        .all(|&c| c == b' ' || c == b'\t')
                })
            }),
            true => None,
        };
        match extend_into {
            None => groups.push(vec![range]),
            Some(group) => group.push(range),
        }
    });
    groups
}

pub struct NoComments;
impl_lint_pass!(NoComments => [NO_COMMENTS]);

impl EarlyLintPass for NoComments {
    fn check_crate(&mut self, early_context: &EarlyContext<'_>, _krate: &ast::Crate) {
        let source_map = early_context.sess().source_map();
        source_map.files().iter().for_each(|file| {
            let path = match &file.name {
                FileName::Real(real) => real.local_path_if_available().to_path_buf(),
                _ => return,
            };
            if !is_local_path(&path) {
                return;
            }
            let Some(src) = file.src.as_ref() else {
                return;
            };
            let base = file.start_pos;
            let comments = find_comments(src);
            group_comments(src, &comments).into_iter().for_each(|group| {
                let parts: Vec<&str> = group.iter().map(|&(lo, hi)| &src[lo..hi]).collect();
                let group_text = parts.join("\n");
                if group_text.lines().any(line_is_justified) {
                    return;
                }
                let Some((&(lo, _), &(_, hi))) = group.first().zip(group.last()) else {
                    return;
                };
                let span =
                    Span::with_root_ctxt(base + BytePos(lo as u32), base + BytePos(hi as u32));
                early_context.opt_span_lint(NO_COMMENTS, Some(span), |diag| {
                    diag.primary_message(
                        "comment must declare its purpose — prefix with `WHY:`, `SAFETY:`, `NOTE:`, `HACK:`, `TODO:`, `FIXME:`, `PERF:`, or include a link or `#1234` ticket ref",
                    );
                });
            });
        });
    }
}
