use rustc_ast::ast;
use rustc_ast::visit::{self, Visitor};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

declare_lint! {
    /// **Warn** — prefer `match` over `if`/`else` chains.
    pub NO_IF_ELSE,
    Warn,
    "prefer match over if/else chains"
}

pub struct NoIfElse;
impl_lint_pass!(NoIfElse => [NO_IF_ELSE]);

struct IfElseVisitor<'cx> {
    early_context: &'cx EarlyContext<'cx>,
    // WHY: when we descend into the `else_expr` of an outer `If`, we're still
    // looking at the same chain — the inner `If` shouldn't emit its own
    // diagnostic. Flag tracked across recursion.
    in_else_if: bool,
}

struct ChainParts {
    arms: Vec<(Span, Span)>,
    final_else: Option<Span>,
    has_let_cond: bool,
}

fn collect_chain(outer_if: &ast::Expr) -> ChainParts {
    let mut arms = Vec::new();
    let mut final_else = None;
    let mut has_let_cond = false;
    let mut current = outer_if;
    loop {
        let ast::ExprKind::If(cond, then_block, Some(else_expr)) = &current.kind else {
            break;
        };
        if matches!(cond.kind, ast::ExprKind::Let(..)) {
            has_let_cond = true;
            break;
        }
        arms.push((cond.span, then_block.span));
        match &else_expr.kind {
            ast::ExprKind::If(..) => current = else_expr,
            _ => {
                final_else = Some(else_expr.span);
                break;
            },
        }
    }
    ChainParts {
        arms,
        final_else,
        has_let_cond,
    }
}

impl IfElseVisitor<'_> {
    fn emit(&self, outer_if: &ast::Expr) {
        let chain = collect_chain(outer_if);
        self.early_context
            .opt_span_lint(NO_IF_ELSE, Some(outer_if.span), |diag| {
                diag.primary_message(
                    "`if`/`else` chain — prefer `match` for exhaustive case analysis",
                );
                let Some(else_span) = chain.final_else else {
                    return;
                };
                if chain.has_let_cond {
                    return;
                }
                let source_map = self.early_context.sess().source_map();
                let arm_texts: Option<Vec<(String, String)>> = chain
                    .arms
                    .iter()
                    .map(|(cond_span, then_span)| {
                        let cond = source_map.span_to_snippet(*cond_span).ok()?;
                        let then = source_map.span_to_snippet(*then_span).ok()?;
                        Some((cond, then))
                    })
                    .collect();
                let Some(arm_texts) = arm_texts else {
                    return;
                };
                let Ok(else_text) = source_map.span_to_snippet(else_span) else {
                    return;
                };
                let mut replacement = String::from("match () {\n");
                arm_texts.iter().for_each(|(cond, then)| {
                    replacement.push_str(&format!("    _ if {cond} => {then},\n"));
                });
                replacement.push_str(&format!("    _ => {else_text},\n}}"));
                diag.span_suggestion(
                    outer_if.span,
                    "rewrite as `match`",
                    replacement,
                    Applicability::MachineApplicable,
                );
            });
    }
}

impl<'ast> Visitor<'ast> for IfElseVisitor<'_> {
    fn visit_expr(&mut self, expr: &'ast ast::Expr) {
        let was_in_chain = self.in_else_if;
        if let ast::ExprKind::If(cond, then_block, Some(else_expr)) = &expr.kind {
            let cond_is_let = matches!(cond.kind, ast::ExprKind::Let(..));
            if !expr.span.from_expansion() && !cond_is_let && !was_in_chain {
                self.emit(expr);
            }
            // Cond and then-block start fresh chains for any nested ifs.
            self.in_else_if = false;
            self.visit_expr(cond);
            self.visit_block(then_block);
            // Only the `else if` continuation propagates chain context.
            self.in_else_if = matches!(else_expr.kind, ast::ExprKind::If(..));
            self.visit_expr(else_expr);
            self.in_else_if = was_in_chain;
            return;
        }
        self.in_else_if = false;
        visit::walk_expr(self, expr);
        self.in_else_if = was_in_chain;
    }
}

impl EarlyLintPass for NoIfElse {
    fn check_crate(&mut self, early_context: &EarlyContext<'_>, crate_root: &ast::Crate) {
        let mut visitor = IfElseVisitor {
            early_context,
            in_else_if: false,
        };
        visit::walk_crate(&mut visitor, crate_root);
    }
}
