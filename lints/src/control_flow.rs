use rustc_ast::ast;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};

declare_lint! {
    /// **Warn** — prefer `match` over `if`/`else` chains.
    pub NO_IF_ELSE,
    Warn,
    "prefer match over if/else chains"
}

pub struct NoIfElse;
impl_lint_pass!(NoIfElse => [NO_IF_ELSE]);

impl EarlyLintPass for NoIfElse {
    fn check_expr(&mut self, early_context: &EarlyContext<'_>, expr: &ast::Expr) {
        if expr.span.from_expansion() {
            return;
        }
        let ast::ExprKind::If(cond, _then, else_opt) = &expr.kind else {
            return;
        };
        if matches!(cond.kind, ast::ExprKind::Let(..)) {
            return;
        }
        if else_opt.is_none() {
            return;
        }
        early_context.opt_span_lint(NO_IF_ELSE, Some(expr.span), |diag| {
            diag.primary_message("`if`/`else` chain — prefer `match` for exhaustive case analysis");
        });
    }
}
