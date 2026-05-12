use rustc_ast::ast;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

/// Returns `Some((index, prev_name, curr_name))` for the first pair of
/// adjacent names that are out of alphabetical order.
fn first_unsorted(names: &[String]) -> Option<(usize, String, String)> {
    names
        .windows(2)
        .enumerate()
        .find_map(|(i, w)| match w[0] > w[1] {
            false => None,
            true => Some((i + 1, w[0].clone(), w[1].clone())),
        })
}

pub struct Msg(pub String);

struct LintEmission<'a> {
    early_context: &'a EarlyContext<'a>,
    lint: &'static rustc_lint::Lint,
    msg: Msg,
    span: Span,
}

fn emit_lint(lint_emission: LintEmission<'_>) {
    let LintEmission {
        early_context,
        lint,
        msg: Msg(msg),
        span,
    } = lint_emission;
    early_context.opt_span_lint(lint, Some(span), |diag| {
        diag.primary_message(msg);
    });
}

declare_lint! {
    /// **Deny** — struct fields must be in alphabetical order.
    pub UNSORTED_STRUCT_FIELDS,
    Deny,
    "struct fields must be in alphabetical order"
}

pub struct UnsortedStructFields;
impl_lint_pass!(UnsortedStructFields => [UNSORTED_STRUCT_FIELDS]);

impl EarlyLintPass for UnsortedStructFields {
    fn check_item(&mut self, early_context: &EarlyContext<'_>, item: &ast::Item) {
        if let ast::ItemKind::Struct(_, _, ref vdata) = item.kind {
            let fields = vdata.fields();
            let names: Vec<String> = fields
                .iter()
                .filter_map(|f| f.ident.map(|id| id.name.to_string()))
                .collect();
            // NOTE: skip tuple structs — those don't have named fields to sort.
            if names.len() != fields.len() || names.len() < 2 {
                return;
            }
            if let Some((idx, prev, curr)) = first_unsorted(&names) {
                emit_lint(LintEmission {
                    early_context,
                    lint: UNSORTED_STRUCT_FIELDS,
                    msg: Msg(format!(
                        "struct field `{curr}` should come before `{prev}` (alphabetical order required)"
                    )),
                    span: fields[idx].span,
                });
            }
        }
    }
}

declare_lint! {
    /// **Deny** — enum variants must be in alphabetical order.
    pub UNSORTED_ENUM_VARIANTS,
    Deny,
    "enum variants must be in alphabetical order"
}

pub struct UnsortedEnumVariants;
impl_lint_pass!(UnsortedEnumVariants => [UNSORTED_ENUM_VARIANTS]);

impl EarlyLintPass for UnsortedEnumVariants {
    fn check_item(&mut self, early_context: &EarlyContext<'_>, item: &ast::Item) {
        if let ast::ItemKind::Enum(_, _, ref enum_def) = item.kind {
            let names: Vec<String> = enum_def
                .variants
                .iter()
                .map(|v| v.ident.name.to_string())
                .collect();
            if names.len() < 2 {
                return;
            }
            if let Some((idx, prev, curr)) = first_unsorted(&names) {
                emit_lint(LintEmission {
                    early_context,
                    lint: UNSORTED_ENUM_VARIANTS,
                    msg: Msg(format!(
                        "enum variant `{curr}` should come before `{prev}` (alphabetical order required)"
                    )),
                    span: enum_def.variants[idx].span,
                });
            }
        }
    }
}

declare_lint! {
    /// **Deny** — match arms must be sorted by pattern text. Wildcard `_` must
    /// always be last.
    pub UNSORTED_MATCH_ARMS,
    Deny,
    "match arms must be sorted by pattern text; wildcard `_` must be last"
}

pub struct UnsortedMatchArms;
impl_lint_pass!(UnsortedMatchArms => [UNSORTED_MATCH_ARMS]);

impl EarlyLintPass for UnsortedMatchArms {
    fn check_expr(&mut self, early_context: &EarlyContext<'_>, expr: &ast::Expr) {
        if expr.span.from_expansion() {
            return;
        }
        if let ast::ExprKind::Match(_, ref arms, ..) = expr.kind {
            if arms.len() < 2 {
                return;
            }

            let source_map = early_context.sess().source_map();

            let arm_keys: Vec<(String, bool, Span)> = arms
                .iter()
                .map(|arm| {
                    let is_wild = matches!(arm.pat.kind, ast::PatKind::Wild);
                    let snippet = source_map
                        .span_to_snippet(arm.pat.span)
                        .unwrap_or_else(|_| "_".into());
                    (snippet, is_wild, arm.pat.span)
                })
                .collect();

            let first_wild_pos = arm_keys.iter().position(|(_, w, _)| *w);
            let arm_after_wild =
                first_wild_pos.and_then(|pos| arm_keys.iter().skip(pos + 1).find(|(_, w, _)| !*w));
            if let Some((snippet, _, span)) = arm_after_wild {
                emit_lint(LintEmission {
                    early_context,
                    lint: UNSORTED_MATCH_ARMS,
                    msg: Msg(format!(
                        "match arm `{snippet}` appears after wildcard `_`; wildcard must be last"
                    )),
                    span: *span,
                });
                return;
            }

            let non_wild: Vec<&(String, bool, Span)> =
                arm_keys.iter().filter(|(_, w, _)| !w).collect();
            let names: Vec<String> = non_wild.iter().map(|(s, _, _)| s.clone()).collect();
            if let Some((idx, prev, curr)) = first_unsorted(&names) {
                emit_lint(LintEmission {
                    early_context,
                    lint: UNSORTED_MATCH_ARMS,
                    msg: Msg(format!(
                        "match arm `{curr}` should come before `{prev}` (alphabetical order required)"
                    )),
                    span: non_wild[idx].2,
                });
            }
        }
    }
}

declare_lint! {
    /// **Deny** — every `mod` declaration in a module must appear before any
    /// `use` statement in that module. `cargo fmt` already orders `use`
    /// statements alphabetically, but it does not enforce the mod/use split.
    pub MOD_AFTER_USE,
    Deny,
    "`mod` declarations must precede `use` statements"
}

pub struct ModAfterUse;
impl_lint_pass!(ModAfterUse => [MOD_AFTER_USE]);

fn check_mod_after_use<T: std::ops::Deref<Target = ast::Item>>(
    early_context: &EarlyContext<'_>,
    items: &[T],
) {
    items
        .iter()
        .filter(|item| !item.span.from_expansion())
        .scan(false, |seen_use, item| match item.kind {
            ast::ItemKind::Mod(..) if *seen_use => Some(Some(item.span)),
            ast::ItemKind::Use(_) => {
                *seen_use = true;
                Some(None)
            },
            _ => Some(None),
        })
        .flatten()
        .for_each(|span| {
            emit_lint(LintEmission {
                early_context,
                lint: MOD_AFTER_USE,
                msg: Msg("`mod` declaration must come before any `use` statement".to_string()),
                span,
            });
        });
}

impl EarlyLintPass for ModAfterUse {
    fn check_crate(&mut self, early_context: &EarlyContext<'_>, crate_root: &ast::Crate) {
        check_mod_after_use(early_context, &crate_root.items);
    }

    fn check_item(&mut self, early_context: &EarlyContext<'_>, item: &ast::Item) {
        if let ast::ItemKind::Mod(_, _, ast::ModKind::Loaded(ref items, ..)) = item.kind {
            check_mod_after_use(early_context, items);
        }
    }
}

declare_lint! {
    /// **Deny** — methods within an `impl` block must be grouped as
    /// (1) constructors / static methods, (2) public methods, (3) private
    /// methods, and alphabetically sorted within each group.
    pub UNSORTED_IMPL_METHODS,
    Deny,
    "impl methods must be grouped (static, public, private) and sorted alphabetically within each group"
}

pub struct UnsortedImplMethods;
impl_lint_pass!(UnsortedImplMethods => [UNSORTED_IMPL_METHODS]);

// WHY: the variant order encodes the precedence used by derived `Ord` at line
// `if curr_group < prev_group` — Static must be the smallest discriminant,
// Private the largest. Alphabetising the variants would invert that ordering
// and silently break the lint's logic.
#[allow(unsorted_enum_variants)]
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum MethodGroup {
    Static = 0,
    Public = 1,
    Private = 2,
}

impl MethodGroup {
    fn label(self) -> &'static str {
        match self {
            MethodGroup::Private => "private",
            MethodGroup::Public => "public",
            MethodGroup::Static => "static",
        }
    }
}

fn classify_fn(fn_box: &ast::Fn, visibility: &ast::Visibility) -> MethodGroup {
    let has_self = fn_box.sig.decl.inputs.first().is_some_and(|p| p.is_self());
    let is_public = matches!(
        visibility.kind,
        ast::VisibilityKind::Public | ast::VisibilityKind::Restricted { .. }
    );
    match (has_self, is_public) {
        (false, _) => MethodGroup::Static,
        (true, false) => MethodGroup::Private,
        (true, true) => MethodGroup::Public,
    }
}

impl EarlyLintPass for UnsortedImplMethods {
    fn check_item(&mut self, early_context: &EarlyContext<'_>, item: &ast::Item) {
        if let ast::ItemKind::Impl(ref impl_block) = item.kind {
            let methods: Vec<(String, MethodGroup, Span)> = impl_block
                .items
                .iter()
                .filter_map(|assoc| match assoc.kind {
                    ast::AssocItemKind::Fn(ref fn_box) => Some((
                        fn_box.ident.name.to_string(),
                        classify_fn(fn_box, &assoc.vis),
                        assoc.span,
                    )),
                    _ => None,
                })
                .collect();

            if methods.len() < 2 {
                return;
            }

            let out_of_order = methods.windows(2).find(|w| w[1].1 < w[0].1);
            if let Some(w) = out_of_order {
                let (prev_name, prev_group, _) = &w[0];
                let (curr_name, curr_group, curr_span) = &w[1];
                emit_lint(LintEmission {
                    early_context,
                    lint: UNSORTED_IMPL_METHODS,
                    msg: Msg(format!(
                        "{} method `{curr_name}` must come before {} method `{prev_name}` (group order: static, public, private)",
                        curr_group.label(),
                        prev_group.label(),
                    )),
                    span: *curr_span,
                });
                return;
            }

            let unsorted_in_group = [
                MethodGroup::Static,
                MethodGroup::Public,
                MethodGroup::Private,
            ]
            .into_iter()
            .find_map(|group| {
                let in_group: Vec<(String, Span)> = methods
                    .iter()
                    .filter(|(_, g, _)| *g == group)
                    .map(|(n, _, s)| (n.clone(), *s))
                    .collect();
                let names: Vec<String> = in_group.iter().map(|(n, _)| n.clone()).collect();
                first_unsorted(&names).map(|(idx, prev, curr)| (group, in_group[idx].1, prev, curr))
            });
            if let Some((group, span, prev, curr)) = unsorted_in_group {
                emit_lint(LintEmission {
                    early_context,
                    lint: UNSORTED_IMPL_METHODS,
                    msg: Msg(format!(
                        "{} method `{curr}` should come before `{prev}` (alphabetical within group)",
                        group.label(),
                    )),
                    span,
                });
            }
        }
    }
}

declare_lint! {
    /// **Deny** — `#[derive(...)]` attributes must list traits in alphabetical
    /// order.
    pub UNSORTED_DERIVES,
    Deny,
    "#[derive(...)] traits must be in alphabetical order"
}

pub struct UnsortedDerives;
impl_lint_pass!(UnsortedDerives => [UNSORTED_DERIVES]);

fn is_local_source_path(path: &std::path::Path) -> bool {
    let s = path.to_string_lossy();
    !s.contains("/.cargo/")
        && !s.contains("/.rustup/")
        && !s.contains("/rustlib/")
        && !s.starts_with("<")
}

// WHY: hand-rolled byte-level scan for `#[derive(...)]` attributes — a state
// machine that tracks paren depth. Iterator combinators can't express the
// stateful "advance to matching `)`" sweep without becoming strictly less
// readable than the imperative form.
#[allow(no_loop, no_if_else, raw_primitive_param)]
fn find_derive_attrs(src: &str) -> Vec<(usize, usize, String)> {
    let bytes = src.as_bytes();
    let needle = b"#[derive(";
    let mut out = Vec::new();
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let start = i;
            let mut j = i + needle.len();
            let mut depth: i32 = 1;
            while j < bytes.len() && depth > 0 {
                match bytes[j] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {},
                }
                j += 1;
            }
            if depth == 0 && j < bytes.len() && bytes[j] == b']' {
                let inner_start = i + needle.len();
                let inner_end = j - 1;
                let inner = &src[inner_start..inner_end];
                out.push((start, j + 1, inner.to_string()));
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

impl EarlyLintPass for UnsortedDerives {
    fn check_crate(&mut self, early_context: &EarlyContext<'_>, _krate: &ast::Crate) {
        let source_map = early_context.sess().source_map();
        source_map.files().iter().for_each(|file| {
            let path = match &file.name {
                rustc_span::FileName::Real(real) => real.local_path_if_available().to_path_buf(),
                _ => return,
            };
            if !is_local_source_path(&path) {
                return;
            }
            let Some(src) = file.src.as_ref() else {
                return;
            };
            let base = file.start_pos;
            find_derive_attrs(src).into_iter().for_each(|(lo, hi, inner)| {
                let names: Vec<String> = inner
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if names.len() < 2 {
                    return;
                }
                if let Some((_idx, prev, curr)) = first_unsorted(&names) {
                    let span = Span::with_root_ctxt(
                        base + rustc_span::BytePos(lo as u32),
                        base + rustc_span::BytePos(hi as u32),
                    );
                    emit_lint(LintEmission {
                        early_context,
                        lint: UNSORTED_DERIVES,
                        msg: Msg(format!(
                            "derive trait `{curr}` should come before `{prev}` (alphabetical order required)"
                        )),
                        span,
                    });
                }
            });
        });
    }
}
