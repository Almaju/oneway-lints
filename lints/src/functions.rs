use rustc_ast::ast;
use rustc_ast::visit::FnKind;
use rustc_ast::NodeId;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

// ---------------------------------------------------------------------------
// NO_NESTED_FUNCTIONS
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Warn** — don't define functions inside other functions.
    pub NO_NESTED_FUNCTIONS,
    Warn,
    "don't define functions inside other functions"
}

pub struct NoNestedFunctions;
impl_lint_pass!(NoNestedFunctions => [NO_NESTED_FUNCTIONS]);

impl EarlyLintPass for NoNestedFunctions {
    fn check_fn(&mut self, cx: &EarlyContext<'_>, kind: FnKind<'_>, _span: Span, _id: NodeId) {
        let FnKind::Fn(_, _, fn_box) = kind else {
            return;
        };
        let Some(body) = fn_box.body.as_ref() else {
            return;
        };
        for stmt in &body.stmts {
            if stmt.span.from_expansion() {
                continue;
            }
            if let ast::StmtKind::Item(item) = &stmt.kind {
                if matches!(item.kind, ast::ItemKind::Fn(..)) {
                    cx.opt_span_lint(NO_NESTED_FUNCTIONS, Some(item.span), |diag| {
                        diag.primary_message(
                            "function defined inside another function — extract to module level",
                        );
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ONE_CONSTRUCTOR_NAME
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — constructors must be named `new`.
    pub ONE_CONSTRUCTOR_NAME,
    Deny,
    "constructors must be named `new`"
}

pub struct OneConstructorName;
impl_lint_pass!(OneConstructorName => [ONE_CONSTRUCTOR_NAME]);

fn returns_self(cx: &EarlyContext<'_>, decl: &ast::FnDecl) -> bool {
    let ast::FnRetTy::Ty(ref ty) = decl.output else {
        return false;
    };
    let Ok(snippet) = cx.sess().source_map().span_to_snippet(ty.span) else {
        return false;
    };
    snippet.trim() == "Self"
}

fn has_self_receiver(decl: &ast::FnDecl) -> bool {
    decl.inputs.first().is_some_and(|p| p.is_self())
}

impl EarlyLintPass for OneConstructorName {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &ast::Item) {
        if item.span.from_expansion() {
            return;
        }
        let ast::ItemKind::Impl(ref impl_block) = item.kind else {
            return;
        };
        if impl_block.of_trait.is_some() {
            return;
        }
        for assoc in &impl_block.items {
            let ast::AssocItemKind::Fn(ref fn_box) = assoc.kind else {
                continue;
            };
            let name = fn_box.ident.name.to_string();
            if name == "new" {
                continue;
            }
            if has_self_receiver(&fn_box.sig.decl) {
                continue;
            }
            if !returns_self(cx, &fn_box.sig.decl) {
                continue;
            }
            cx.opt_span_lint(ONE_CONSTRUCTOR_NAME, Some(assoc.span), |diag| {
                diag.primary_message(format!(
                    "constructor `{name}` must be named `new` (returns `Self`, no `self` receiver)"
                ));
            });
        }
    }
}
