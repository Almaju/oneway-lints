use std::ops::Deref;

use rustc_ast::ast;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

use crate::path_ext::PathExt;

trait NamesExt {
    /// Returns `Some((index, prev_name, curr_name))` for the first pair of
    /// adjacent names that are out of alphabetical order.
    fn first_unsorted(&self) -> Option<(usize, String, String)>;
}

impl NamesExt for [String] {
    fn first_unsorted(&self) -> Option<(usize, String, String)> {
        self.windows(2)
            .enumerate()
            .find_map(|(i, w)| match w[0] > w[1] {
                false => None,
                true => Some((i + 1, w[0].clone(), w[1].clone())),
            })
    }
}

pub struct Msg(pub String);

struct LintEmission<'a> {
    early_context: &'a EarlyContext<'a>,
    lint: &'static rustc_lint::Lint,
    msg: Msg,
    span: Span,
}

impl LintEmission<'_> {
    fn emit(self) {
        let LintEmission {
            early_context,
            lint,
            msg: Msg(msg),
            span,
        } = self;
        early_context.opt_span_lint(lint, Some(span), |diag| {
            diag.primary_message(msg);
        });
    }
}

declare_lint! {
    /// **Deny** — struct fields must be in alphabetical order.
    pub UNSORTED_STRUCT_FIELDS,
    Deny,
    "struct fields must be in alphabetical order"
}

pub struct UnsortedStructFields;
impl_lint_pass!(UnsortedStructFields => [UNSORTED_STRUCT_FIELDS]);

trait FieldDefExt {
    fn full_span(&self) -> Span;
}

impl FieldDefExt for ast::FieldDef {
    fn full_span(&self) -> Span {
        let attr_lo = self
            .attrs
            .iter()
            .map(|a| a.span.lo())
            .min()
            .unwrap_or(self.span.lo());
        self.span.with_lo(attr_lo)
    }
}

trait AttrsExt {
    fn has_repr_attr(&self) -> bool;
}

impl AttrsExt for [ast::Attribute] {
    fn has_repr_attr(&self) -> bool {
        self.iter()
            .any(|attr| attr.ident().is_some_and(|id| id.name.as_str() == "repr"))
    }
}

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
            if let Some((idx, prev, curr)) = names.first_unsorted() {
                let span = fields[idx].span;
                let msg = format!(
                    "struct field `{curr}` should come before `{prev}` (alphabetical order required)"
                );
                // WHY: skip autofix when `#[repr(...)]` is present — field
                // order is load-bearing for FFI and packed layouts. We still
                // emit the diagnostic so the author can decide.
                if item.attrs.has_repr_attr() {
                    LintEmission {
                        early_context,
                        lint: UNSORTED_STRUCT_FIELDS,
                        msg: Msg(msg),
                        span,
                    }
                    .emit();
                    return;
                }
                let source_map = early_context.sess().source_map();
                let full_spans: Vec<Span> = fields.iter().map(|f| f.full_span()).collect();
                let texts: Vec<String> = full_spans
                    .iter()
                    .map(|s| source_map.span_to_snippet(*s).unwrap_or_default())
                    .collect();
                let mut indices: Vec<usize> = (0..fields.len()).collect();
                indices.sort_by(|&i, &j| names[i].cmp(&names[j]));
                let parts: Vec<(Span, String)> = (0..fields.len())
                    .map(|i| (full_spans[i], texts[indices[i]].clone()))
                    .collect();
                early_context.opt_span_lint(UNSORTED_STRUCT_FIELDS, Some(span), |diag| {
                    diag.primary_message(msg);
                    diag.multipart_suggestion(
                        "sort the struct fields alphabetically",
                        parts,
                        Applicability::MachineApplicable,
                    );
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
            if let Some((idx, prev, curr)) = names.first_unsorted() {
                let span = enum_def.variants[idx].span;
                let msg = format!(
                    "enum variant `{curr}` should come before `{prev}` (alphabetical order required)"
                );
                // WHY: skip autofix when variant order is load-bearing —
                // derived Ord/PartialOrd/Hash compare by declaration order,
                // and explicit discriminants signal intent the rule shouldn't
                // override. The diagnostic still fires so the author can
                // decide whether to disable the rule or accept the change.
                // WHY: `#[derive(...)]` attrs aren't in `item.attrs` at the
                // EarlyLintPass stage — the macro machinery has already
                // claimed them. Probe the source text immediately preceding
                // the item to find leading derive attrs and check whether
                // they include any of the order-sensitive traits.
                let source_map = early_context.sess().source_map();
                let probe_lo = rustc_span::BytePos(item.span.lo().0.saturating_sub(256));
                let probe_text = source_map
                    .span_to_snippet(item.span.with_lo(probe_lo))
                    .unwrap_or_default();
                let has_order_derive = probe_text.contains("#[derive")
                    && probe_text
                        .split(|c: char| !c.is_alphanumeric() && c != '_')
                        .any(|tok| matches!(tok, "Hash" | "Ord" | "PartialOrd"));
                let has_explicit_discriminant =
                    enum_def.variants.iter().any(|v| v.disr_expr.is_some());
                if has_order_derive || has_explicit_discriminant {
                    LintEmission {
                        early_context,
                        lint: UNSORTED_ENUM_VARIANTS,
                        msg: Msg(msg),
                        span,
                    }
                    .emit();
                    return;
                }
                let source_map = early_context.sess().source_map();
                let full_spans: Vec<Span> =
                    enum_def.variants.iter().map(|v| v.full_span()).collect();
                let texts: Vec<String> = full_spans
                    .iter()
                    .map(|s| source_map.span_to_snippet(*s).unwrap_or_default())
                    .collect();
                let mut indices: Vec<usize> = (0..enum_def.variants.len()).collect();
                indices.sort_by(|&i, &j| names[i].cmp(&names[j]));
                let parts: Vec<(Span, String)> = (0..enum_def.variants.len())
                    .map(|i| (full_spans[i], texts[indices[i]].clone()))
                    .collect();
                early_context.opt_span_lint(UNSORTED_ENUM_VARIANTS, Some(span), |diag| {
                    diag.primary_message(msg);
                    diag.multipart_suggestion(
                        "sort the enum variants alphabetically",
                        parts,
                        Applicability::MachineApplicable,
                    );
                });
            }
        }
    }
}

trait VariantExt {
    fn full_span(&self) -> Span;
}

impl VariantExt for ast::Variant {
    fn full_span(&self) -> Span {
        let attr_lo = self
            .attrs
            .iter()
            .map(|a| a.span.lo())
            .min()
            .unwrap_or(self.span.lo());
        self.span.with_lo(attr_lo)
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

trait ArmExt {
    fn full_span(&self) -> Span;
}

impl ArmExt for ast::Arm {
    fn full_span(&self) -> Span {
        let attr_lo = self
            .attrs
            .iter()
            .map(|a| a.span.lo())
            .min()
            .unwrap_or(self.span.lo());
        self.span.with_lo(attr_lo)
    }
}

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
                // WHY: no autofix — moving the wildcard arm could collapse
                // multiple wildcard-like arms or change matching priority for
                // arms with overlapping patterns. Author's call.
                LintEmission {
                    early_context,
                    lint: UNSORTED_MATCH_ARMS,
                    msg: Msg(format!(
                        "match arm `{snippet}` appears after wildcard `_`; wildcard must be last"
                    )),
                    span: *span,
                }
                .emit();
                return;
            }

            let non_wild: Vec<&(String, bool, Span)> =
                arm_keys.iter().filter(|(_, w, _)| !w).collect();
            let names: Vec<String> = non_wild.iter().map(|(s, _, _)| s.clone()).collect();
            if let Some((idx, prev, curr)) = names.first_unsorted() {
                let span = non_wild[idx].2;
                let msg = format!(
                    "match arm `{curr}` should come before `{prev}` (alphabetical order required)"
                );
                // WHY: skip autofix when any arm has a guard — guards can
                // overlap with later patterns, so swapping arm order could
                // change which arm matches.
                let has_guard = arms.iter().any(|a| a.guard.is_some());
                if has_guard {
                    LintEmission {
                        early_context,
                        lint: UNSORTED_MATCH_ARMS,
                        msg: Msg(msg),
                        span,
                    }
                    .emit();
                    return;
                }
                let full_spans: Vec<Span> = arms.iter().map(|a| a.full_span()).collect();
                let texts: Vec<String> = full_spans
                    .iter()
                    .map(|s| source_map.span_to_snippet(*s).unwrap_or_default())
                    .collect();
                let non_wild_positions: Vec<usize> = arms
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| !matches!(a.pat.kind, ast::PatKind::Wild))
                    .map(|(i, _)| i)
                    .collect();
                let mut sorted_positions = non_wild_positions.clone();
                sorted_positions.sort_by(|&i, &j| arm_keys[i].0.cmp(&arm_keys[j].0));
                let parts: Vec<(Span, String)> = non_wild_positions
                    .iter()
                    .zip(&sorted_positions)
                    .map(|(&dest, &src)| (full_spans[dest], texts[src].clone()))
                    .collect();
                early_context.opt_span_lint(UNSORTED_MATCH_ARMS, Some(span), |diag| {
                    diag.primary_message(msg);
                    diag.multipart_suggestion(
                        "sort the match arms alphabetically",
                        parts,
                        Applicability::MachineApplicable,
                    );
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

trait ItemsExt {
    fn check_mod_after_use(&self, early_context: &EarlyContext<'_>);
}

impl<T: Deref<Target = ast::Item>> ItemsExt for [T] {
    fn check_mod_after_use(&self, early_context: &EarlyContext<'_>) {
        let mod_use_indices: Vec<usize> = self
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.span.from_expansion())
            .filter(|(_, item)| matches!(item.kind, ast::ItemKind::Mod(..) | ast::ItemKind::Use(_)))
            .map(|(i, _)| i)
            .collect();

        let first_misplaced = mod_use_indices.iter().enumerate().find(|(seq_idx, &i)| {
            if !matches!(self[i].kind, ast::ItemKind::Mod(..)) {
                return false;
            }
            mod_use_indices[..*seq_idx]
                .iter()
                .any(|&j| matches!(self[j].kind, ast::ItemKind::Use(_)))
        });
        let Some((_, &first_misplaced_index)) = first_misplaced else {
            return;
        };

        let msg = "`mod` declaration must come before any `use` statement".to_string();
        let span = self[first_misplaced_index].span;

        let source_map = early_context.sess().source_map();
        let texts: Vec<String> = mod_use_indices
            .iter()
            .map(|&i| source_map.span_to_snippet(self[i].span).unwrap_or_default())
            .collect();
        let mods_first: Vec<usize> = mod_use_indices
            .iter()
            .copied()
            .filter(|&i| matches!(self[i].kind, ast::ItemKind::Mod(..)))
            .chain(
                mod_use_indices
                    .iter()
                    .copied()
                    .filter(|&i| matches!(self[i].kind, ast::ItemKind::Use(_))),
            )
            .collect();
        let parts: Vec<(Span, String)> = mod_use_indices
            .iter()
            .zip(&mods_first)
            .map(|(&dest_idx, &src_idx)| {
                let src_pos = mod_use_indices
                    .iter()
                    .position(|&i| i == src_idx)
                    .unwrap_or(0);
                (self[dest_idx].span, texts[src_pos].clone())
            })
            .collect();

        early_context.opt_span_lint(MOD_AFTER_USE, Some(span), |diag| {
            diag.primary_message(msg);
            diag.multipart_suggestion(
                "move all `mod` declarations before `use` statements",
                parts,
                Applicability::MachineApplicable,
            );
        });
    }
}

impl EarlyLintPass for ModAfterUse {
    fn check_crate(&mut self, early_context: &EarlyContext<'_>, crate_root: &ast::Crate) {
        crate_root.items.check_mod_after_use(early_context);
    }

    fn check_item(&mut self, early_context: &EarlyContext<'_>, item: &ast::Item) {
        if let ast::ItemKind::Mod(_, _, ast::ModKind::Loaded(ref items, ..)) = item.kind {
            items.check_mod_after_use(early_context);
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

trait FnExt {
    fn classify(&self, visibility: &ast::Visibility) -> MethodGroup;
}

impl FnExt for ast::Fn {
    fn classify(&self, visibility: &ast::Visibility) -> MethodGroup {
        let has_self = self.sig.decl.inputs.first().is_some_and(|p| p.is_self());
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
}

trait AssocItemExt {
    fn full_span(&self) -> Span;
}

impl AssocItemExt for ast::AssocItem {
    fn full_span(&self) -> Span {
        let attr_lo = self
            .attrs
            .iter()
            .map(|a| a.span.lo())
            .min()
            .unwrap_or(self.span.lo());
        self.span.with_lo(attr_lo)
    }
}

impl EarlyLintPass for UnsortedImplMethods {
    fn check_item(&mut self, early_context: &EarlyContext<'_>, item: &ast::Item) {
        if let ast::ItemKind::Impl(ref impl_block) = item.kind {
            let method_positions: Vec<usize> = impl_block
                .items
                .iter()
                .enumerate()
                .filter_map(|(i, assoc)| match assoc.kind {
                    ast::AssocItemKind::Fn(_) => Some(i),
                    _ => None,
                })
                .collect();

            if method_positions.len() < 2 {
                return;
            }

            let methods: Vec<(String, MethodGroup, Span)> = method_positions
                .iter()
                .filter_map(|&i| {
                    let assoc = &impl_block.items[i];
                    let ast::AssocItemKind::Fn(ref fn_box) = assoc.kind else {
                        return None;
                    };
                    Some((
                        fn_box.ident.name.to_string(),
                        fn_box.classify(&assoc.vis),
                        assoc.full_span(),
                    ))
                })
                .collect();

            let mut sorted_indices: Vec<usize> = (0..methods.len()).collect();
            sorted_indices.sort_by(|&i, &j| {
                (methods[i].1, &methods[i].0).cmp(&(methods[j].1, &methods[j].0))
            });

            let first_diff = sorted_indices
                .iter()
                .enumerate()
                .find(|(actual_idx, &sorted_idx)| *actual_idx != sorted_idx);
            let Some((actual_idx, &sorted_idx)) = first_diff else {
                return;
            };

            let actual = &methods[actual_idx];
            let expected = &methods[sorted_idx];
            let msg = match expected.1.cmp(&actual.1) {
                std::cmp::Ordering::Equal => format!(
                    "{} method `{}` should come before `{}` (alphabetical within group)",
                    actual.1.label(),
                    expected.0,
                    actual.0,
                ),
                std::cmp::Ordering::Greater => format!(
                    "{} method `{}` should come before `{}` (alphabetical within group)",
                    actual.1.label(),
                    expected.0,
                    actual.0,
                ),
                std::cmp::Ordering::Less => format!(
                    "{} method `{}` must come before {} method `{}` (group order: static, public, private)",
                    expected.1.label(),
                    expected.0,
                    actual.1.label(),
                    actual.0,
                ),
            };

            let source_map = early_context.sess().source_map();
            let texts: Vec<String> = methods
                .iter()
                .map(|(_, _, span)| source_map.span_to_snippet(*span).unwrap_or_default())
                .collect();
            let parts: Vec<(Span, String)> = (0..methods.len())
                .map(|i| (methods[i].2, texts[sorted_indices[i]].clone()))
                .collect();

            early_context.opt_span_lint(UNSORTED_IMPL_METHODS, Some(actual.2), |diag| {
                diag.primary_message(msg);
                diag.multipart_suggestion(
                    "sort the impl methods (static → public → private, alphabetical within each group)",
                    parts,
                    Applicability::MachineApplicable,
                );
            });
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

trait SrcExt {
    fn find_derive_attrs(&self) -> Vec<(usize, usize, String)>;
}

// WHY: hand-rolled byte-level scan for `#[derive(...)]` attributes — a state
// machine that tracks paren depth. Iterator combinators can't express the
// stateful "advance to matching `)`" sweep without becoming strictly less
// readable than the imperative form.
#[allow(no_if_else)]
impl SrcExt for str {
    fn find_derive_attrs(&self) -> Vec<(usize, usize, String)> {
        let bytes = self.as_bytes();
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
                    let inner = &self[inner_start..inner_end];
                    out.push((start, j + 1, inner.to_string()));
                    i = j + 1;
                    continue;
                }
            }
            i += 1;
        }
        out
    }
}

impl EarlyLintPass for UnsortedDerives {
    fn check_crate(&mut self, early_context: &EarlyContext<'_>, _krate: &ast::Crate) {
        let source_map = early_context.sess().source_map();
        source_map.files().iter().for_each(|file| {
            let path = match &file.name {
                rustc_span::FileName::Real(real) => real.local_path_if_available().to_path_buf(),
                _ => return,
            };
            if !path.is_local_source() {
                return;
            }
            let Some(src) = file.src.as_ref() else {
                return;
            };
            let base = file.start_pos;
            src.find_derive_attrs().into_iter().for_each(|(lo, hi, inner)| {
                let names: Vec<String> = inner
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if names.len() < 2 {
                    return;
                }
                if let Some((_idx, prev, curr)) = names.first_unsorted() {
                    let span = Span::with_root_ctxt(
                        base + rustc_span::BytePos(lo as u32),
                        base + rustc_span::BytePos(hi as u32),
                    );
                    let mut sorted = names.clone();
                    sorted.sort();
                    let replacement = format!("#[derive({})]", sorted.join(", "));
                    early_context.opt_span_lint(UNSORTED_DERIVES, Some(span), |diag| {
                        diag.primary_message(format!(
                            "derive trait `{curr}` should come before `{prev}` (alphabetical order required)"
                        ));
                        diag.span_suggestion(
                            span,
                            "sort the derive list alphabetically",
                            replacement,
                            Applicability::MachineApplicable,
                        );
                    });
                }
            });
        });
    }
}
