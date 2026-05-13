use std::collections::HashMap;

use rustc_ast::ast;
use rustc_ast::visit::{self, FnCtxt, FnKind, Visitor};
use rustc_ast::NodeId;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

declare_lint! {
    /// **Warn** — don't define functions inside other functions.
    pub NO_NESTED_FUNCTIONS,
    Warn,
    "don't define functions inside other functions"
}

pub struct NoNestedFunctions;
impl_lint_pass!(NoNestedFunctions => [NO_NESTED_FUNCTIONS]);

impl EarlyLintPass for NoNestedFunctions {
    fn check_fn(
        &mut self,
        early_context: &EarlyContext<'_>,
        fn_kind: FnKind<'_>,
        outer_span: Span,
        _id: NodeId,
    ) {
        let FnKind::Fn(fn_ctxt, _, fn_box) = fn_kind else {
            return;
        };
        let Some(body) = fn_box.body.as_ref() else {
            return;
        };
        // WHY: autofix is only valid when the outer fn is at module level.
        // Hoisting from inside an `impl` method would land the new fn
        // INSIDE the impl block, which isn't what "module level" means and
        // would compile to a different name path.
        let autofix_allowed = matches!(fn_ctxt, FnCtxt::Free);
        let source_map = early_context.sess().source_map();
        body.stmts
            .iter()
            .filter(|stmt| !stmt.span.from_expansion())
            .filter_map(|stmt| match &stmt.kind {
                ast::StmtKind::Item(item) if matches!(item.kind, ast::ItemKind::Fn(..)) => {
                    Some((stmt.span, item.span))
                },
                _ => None,
            })
            .for_each(|(stmt_span, item_span)| {
                early_context.opt_span_lint(NO_NESTED_FUNCTIONS, Some(item_span), |diag| {
                    diag.primary_message(
                        "function defined inside another function — extract to module level",
                    );
                    if !autofix_allowed {
                        return;
                    }
                    let Ok(inner_text) = source_map.span_to_snippet(item_span) else {
                        return;
                    };
                    // WHY: Rust's nested fns can't close over the outer fn's
                    // locals or generics — hoisting is semantically a no-op
                    // unless the inner fn's name collides with something at
                    // module scope. Marked `MaybeIncorrect` to cover that
                    // collision case.
                    diag.multipart_suggestion(
                        "hoist the nested function to module level",
                        vec![
                            (stmt_span, String::new()),
                            (outer_span.shrink_to_hi(), format!("\n\n{inner_text}")),
                        ],
                        Applicability::MachineApplicable,
                    );
                });
            });
    }
}

declare_lint! {
    /// **Deny** — constructors must not use near-synonyms for `new` such as
    /// `create`, `build`, `init`, `make`, or `construct`. Other descriptive
    /// constructor names (`from_string`, `with_capacity`, role-discriminating
    /// names like `user`/`system`/`assistant`, etc.) are allowed.
    pub ONE_CONSTRUCTOR_NAME,
    Deny,
    "constructors must not use the synonyms `create`/`build`/`init`/`make`/`construct` — use `new` or a descriptive name"
}

pub struct OneConstructorName;
impl_lint_pass!(OneConstructorName => [ONE_CONSTRUCTOR_NAME]);

const FORBIDDEN_NAMES: &[&str] = &["build", "construct", "create", "init", "make"];

fn returns_self(early_context: &EarlyContext<'_>, fn_decl: &ast::FnDecl) -> bool {
    let ast::FnRetTy::Ty(ref ty) = fn_decl.output else {
        return false;
    };
    let Ok(snippet) = early_context.sess().source_map().span_to_snippet(ty.span) else {
        return false;
    };
    snippet.trim() == "Self"
}

fn has_self_receiver(fn_decl: &ast::FnDecl) -> bool {
    fn_decl.inputs.first().is_some_and(|p| p.is_self())
}

struct Candidate {
    autofixable: bool,
    diag_span: Span,
    ident_span: Span,
    method_name: String,
    type_name: String,
}

fn self_ty_name(ty: &ast::Ty) -> Option<String> {
    match &ty.kind {
        ast::TyKind::Path(_, path) => path.segments.last().map(|s| s.ident.name.to_string()),
        _ => None,
    }
}

/// Walk every `impl` block and collect renameable constructors. A
/// `(type_name, method_name)` pair is autofixable when:
///   - exactly one method on `type_name` carries a forbidden constructor name
///   - `type_name` has no method already called `new` (would collide)
fn collect_candidates(
    early_context: &EarlyContext<'_>,
    crate_root: &ast::Crate,
) -> Vec<Candidate> {
    let mut by_type: HashMap<String, Vec<Candidate>> = HashMap::new();
    let mut existing_new_by_type: HashMap<String, bool> = HashMap::new();
    crate_root.items.iter().for_each(|item| {
        scan_item(early_context, item, &mut by_type, &mut existing_new_by_type);
    });
    let mut out: Vec<Candidate> = by_type
        .into_iter()
        .flat_map(|(type_name, mut entries)| {
            let has_new = existing_new_by_type.get(&type_name).copied().unwrap_or(false);
            let autofixable = !has_new && entries.len() == 1;
            entries.iter_mut().for_each(|c| c.autofixable = autofixable);
            entries
        })
        .collect();
    // WHY: HashMap iteration order is non-deterministic; sort by source
    // position so the diagnostic stream (and the UI test stderr) is stable.
    out.sort_by_key(|c| c.diag_span.lo());
    out
}

fn scan_item(
    early_context: &EarlyContext<'_>,
    item: &ast::Item,
    by_type: &mut HashMap<String, Vec<Candidate>>,
    existing_new_by_type: &mut HashMap<String, bool>,
) {
    match &item.kind {
        ast::ItemKind::Impl(impl_block) if impl_block.of_trait.is_none() => {
            if item.span.from_expansion() {
                return;
            }
            let Some(type_name) = self_ty_name(&impl_block.self_ty) else {
                return;
            };
            impl_block.items.iter().for_each(|assoc| {
                let ast::AssocItemKind::Fn(fn_box) = &assoc.kind else {
                    return;
                };
                if has_self_receiver(&fn_box.sig.decl) {
                    return;
                }
                if !returns_self(early_context, &fn_box.sig.decl) {
                    return;
                }
                let method_name = fn_box.ident.name.to_string();
                if method_name == "new" {
                    existing_new_by_type.insert(type_name.clone(), true);
                    return;
                }
                if !FORBIDDEN_NAMES.contains(&method_name.as_str()) {
                    return;
                }
                by_type.entry(type_name.clone()).or_default().push(Candidate {
                    autofixable: false,
                    diag_span: assoc.span,
                    ident_span: fn_box.ident.span,
                    method_name,
                    type_name: type_name.clone(),
                });
            });
        },
        ast::ItemKind::Mod(_, _, ast::ModKind::Loaded(items, ..)) => {
            items.iter().for_each(|child| {
                scan_item(early_context, child, by_type, existing_new_by_type);
            });
        },
        _ => {},
    }
}

/// Collects `<Type>::<method>` path-expression call sites for each
/// `(type_name, method_name)` of interest. Method-call expressions like
/// `instance.create()` aren't tracked — the lint already filters out
/// methods with a `self` receiver, so all renameable constructors are
/// reached via the `Type::method` path form.
struct CallSiteVisitor<'a> {
    hits: HashMap<(String, String), Vec<Span>>,
    interest: &'a HashMap<(String, String), ()>,
}

impl<'ast> Visitor<'ast> for CallSiteVisitor<'_> {
    fn visit_expr(&mut self, expr: &'ast ast::Expr) {
        if let ast::ExprKind::Path(None, path) = &expr.kind {
            if path.segments.len() >= 2 {
                let len = path.segments.len();
                let type_seg = &path.segments[len - 2];
                let method_seg = &path.segments[len - 1];
                let type_name = type_seg.ident.name.to_string();
                let method_name = method_seg.ident.name.to_string();
                let key = (type_name, method_name);
                if self.interest.contains_key(&key) {
                    self.hits
                        .entry(key)
                        .or_default()
                        .push(method_seg.ident.span);
                }
            }
        }
        visit::walk_expr(self, expr);
    }
}

impl EarlyLintPass for OneConstructorName {
    fn check_crate(&mut self, early_context: &EarlyContext<'_>, crate_root: &ast::Crate) {
        let candidates = collect_candidates(early_context, crate_root);
        if candidates.is_empty() {
            return;
        }
        let interest: HashMap<(String, String), ()> = candidates
            .iter()
            .filter(|c| c.autofixable)
            .map(|c| ((c.type_name.clone(), c.method_name.clone()), ()))
            .collect();
        let mut call_visitor = CallSiteVisitor {
            hits: HashMap::new(),
            interest: &interest,
        };
        visit::walk_crate(&mut call_visitor, crate_root);
        candidates.into_iter().for_each(|candidate| {
            let Candidate {
                diag_span,
                ident_span,
                method_name,
                type_name,
                autofixable,
            } = candidate;
            let msg = format!(
                "constructor `{method_name}` is a synonym for `new` — rename to `new` or use a descriptive name (e.g. `from_string`, `with_capacity`)"
            );
            let key = (type_name, method_name);
            early_context.opt_span_lint(ONE_CONSTRUCTOR_NAME, Some(diag_span), |diag| {
                diag.primary_message(msg);
                if !autofixable {
                    return;
                }
                let mut parts: Vec<(Span, String)> = vec![(ident_span, "new".to_string())];
                if let Some(call_spans) = call_visitor.hits.get(&key) {
                    call_spans.iter().for_each(|span| {
                        parts.push((*span, "new".to_string()));
                    });
                }
                diag.multipart_suggestion(
                    "rename the constructor (and its call sites) to `new`",
                    parts,
                    Applicability::MachineApplicable,
                );
            });
        });
    }
}
