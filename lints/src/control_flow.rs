use rustc_ast::ast;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};

// ---------------------------------------------------------------------------
// NO_LOOP
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Deny** — don't use `loop`, `while`, or `for`. Use iterators instead.
    pub NO_LOOP,
    Deny,
    "don't use loop/while/for — use iterators"
}

pub struct NoLoop;
impl_lint_pass!(NoLoop => [NO_LOOP]);

impl EarlyLintPass for NoLoop {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &ast::Expr) {
        if expr.span.from_expansion() {
            return;
        }
        let kind_name = match expr.kind {
            ast::ExprKind::Loop(..) => "loop",
            ast::ExprKind::While(..) => "while",
            ast::ExprKind::ForLoop { .. } => "for",
            _ => return,
        };
        cx.opt_span_lint(NO_LOOP, Some(expr.span), |diag| {
            diag.primary_message(format!(
                "`{kind_name}` is forbidden — use iterators and combinators instead"
            ));
        });
    }
}

// ---------------------------------------------------------------------------
// NO_IF_ELSE
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Warn** — prefer `match` over `if`/`else` chains.
    pub NO_IF_ELSE,
    Warn,
    "prefer match over if/else chains"
}

pub struct NoIfElse;
impl_lint_pass!(NoIfElse => [NO_IF_ELSE]);

impl EarlyLintPass for NoIfElse {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &ast::Expr) {
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
        cx.opt_span_lint(NO_IF_ELSE, Some(expr.span), |diag| {
            diag.primary_message("`if`/`else` chain — prefer `match` for exhaustive case analysis");
        });
    }
}
