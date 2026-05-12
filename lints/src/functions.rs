use rustc_ast::ast;
use rustc_ast::visit::FnKind;
use rustc_ast::NodeId;
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
        _span: Span,
        _id: NodeId,
    ) {
        let FnKind::Fn(_, _, fn_box) = fn_kind else {
            return;
        };
        let Some(body) = fn_box.body.as_ref() else {
            return;
        };
        body.stmts
            .iter()
            .filter(|stmt| !stmt.span.from_expansion())
            .filter_map(|stmt| match &stmt.kind {
                ast::StmtKind::Item(item) if matches!(item.kind, ast::ItemKind::Fn(..)) => {
                    Some(item.span)
                },
                _ => None,
            })
            .for_each(|span| {
                early_context.opt_span_lint(NO_NESTED_FUNCTIONS, Some(span), |diag| {
                    diag.primary_message(
                        "function defined inside another function — extract to module level",
                    );
                });
            });
    }
}

declare_lint! {
    /// **Deny** — constructors must be named `new`.
    pub ONE_CONSTRUCTOR_NAME,
    Deny,
    "constructors must be named `new`"
}

pub struct OneConstructorName;
impl_lint_pass!(OneConstructorName => [ONE_CONSTRUCTOR_NAME]);

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

impl EarlyLintPass for OneConstructorName {
    fn check_item(&mut self, early_context: &EarlyContext<'_>, item: &ast::Item) {
        if item.span.from_expansion() {
            return;
        }
        let ast::ItemKind::Impl(ref impl_block) = item.kind else {
            return;
        };
        if impl_block.of_trait.is_some() {
            return;
        }
        impl_block
            .items
            .iter()
            .filter_map(|assoc| match assoc.kind {
                ast::AssocItemKind::Fn(ref fn_box) => Some((assoc, fn_box)),
                _ => None,
            })
            .filter(|(_, fn_box)| fn_box.ident.name.as_str() != "new")
            .filter(|(_, fn_box)| !has_self_receiver(&fn_box.sig.decl))
            .filter(|(_, fn_box)| returns_self(early_context, &fn_box.sig.decl))
            .for_each(|(assoc, fn_box)| {
                let name = fn_box.ident.name.to_string();
                early_context.opt_span_lint(ONE_CONSTRUCTOR_NAME, Some(assoc.span), |diag| {
                    diag.primary_message(format!(
                        "constructor `{name}` must be named `new` (returns `Self`, no `self` receiver)"
                    ));
                });
            });
    }
}
