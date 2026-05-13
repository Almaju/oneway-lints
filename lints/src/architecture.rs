use std::collections::HashMap;
use std::collections::HashSet;

use rustc_ast::ast;
use rustc_ast::visit::{self, Visitor};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

declare_lint! {
    /// **Deny** — a public method in an inherent `impl` must not call another
    /// public method on `self`. Public-to-public composition is workflow
    /// (use-case) logic; extract it to a dedicated struct that depends on the
    /// type. Private helper methods on `self` remain free to use — they're
    /// internal organization, not API composition.
    pub NO_SELF_ORCHESTRATION,
    Deny,
    "public method orchestrates other public methods on self — extract to a use-case struct"
}

pub struct NoSelfOrchestration;
impl_lint_pass!(NoSelfOrchestration => [NO_SELF_ORCHESTRATION]);

trait ImplExt {
    fn target_simple_name(&self) -> Option<String>;
}

impl ImplExt for ast::Impl {
    fn target_simple_name(&self) -> Option<String> {
        let ast::TyKind::Path(_, ref path) = self.self_ty.kind else {
            return None;
        };
        path.segments.last().map(|seg| seg.ident.name.to_string())
    }
}

trait VisibilityExt {
    fn is_pub(&self) -> bool;
}

impl VisibilityExt for ast::Visibility {
    fn is_pub(&self) -> bool {
        matches!(
            self.kind,
            ast::VisibilityKind::Public | ast::VisibilityKind::Restricted { .. }
        )
    }
}

trait ExprExt {
    fn is_self_receiver(&self) -> bool;
}

impl ExprExt for ast::Expr {
    fn is_self_receiver(&self) -> bool {
        match &self.kind {
            ast::ExprKind::Path(_, path) => {
                path.segments.len() == 1 && path.segments[0].ident.name.as_str() == "self"
            },
            _ => false,
        }
    }
}

struct CollectVisitor {
    pub_methods_by_type: HashMap<String, HashSet<String>>,
}

impl<'ast> Visitor<'ast> for CollectVisitor {
    fn visit_item(&mut self, item: &'ast ast::Item) {
        if let ast::ItemKind::Impl(ref impl_block) = item.kind {
            if impl_block.of_trait.is_none() {
                if let Some(type_name) = impl_block.target_simple_name() {
                    let entry = self.pub_methods_by_type.entry(type_name).or_default();
                    impl_block.items.iter().for_each(|assoc| {
                        if let ast::AssocItemKind::Fn(ref fn_box) = assoc.kind {
                            if assoc.vis.is_pub() {
                                entry.insert(fn_box.ident.name.to_string());
                            }
                        }
                    });
                }
            }
        }
        visit::walk_item(self, item);
    }
}

struct CheckVisitor<'a, 'cx> {
    early_context: &'cx EarlyContext<'cx>,
    pub_methods_by_type: &'a HashMap<String, HashSet<String>>,
}

impl<'ast> Visitor<'ast> for CheckVisitor<'_, '_> {
    fn visit_item(&mut self, item: &'ast ast::Item) {
        if !item.span.from_expansion() {
            if let ast::ItemKind::Impl(ref impl_block) = item.kind {
                if impl_block.of_trait.is_none() {
                    if let Some(type_name) = impl_block.target_simple_name() {
                        if let Some(pub_methods) = self.pub_methods_by_type.get(&type_name) {
                            impl_block.items.iter().for_each(|assoc| {
                                if let ast::AssocItemKind::Fn(ref fn_box) = assoc.kind {
                                    if assoc.vis.is_pub() {
                                        if let Some(block) = fn_box.body.as_ref() {
                                            let method_name = fn_box.ident.name.to_string();
                                            let mut visitor = OrchestrationVisitor {
                                                offenders: Vec::new(),
                                                pub_methods,
                                            };
                                            visit::walk_block(&mut visitor, block);
                                            visitor.offenders.iter().for_each(|(called, span)| {
                                                self.early_context.opt_span_lint(
                                                    NO_SELF_ORCHESTRATION,
                                                    Some(*span),
                                                    |diag| {
                                                        diag.primary_message(format!(
                                                            "public method `{method_name}` calls public method `self.{called}()` — extract this composition to a dedicated use-case struct that depends on the type"
                                                        ));
                                                    },
                                                );
                                            });
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }
        visit::walk_item(self, item);
    }
}

struct OrchestrationVisitor<'a> {
    offenders: Vec<(String, Span)>,
    pub_methods: &'a HashSet<String>,
}

impl<'ast> Visitor<'ast> for OrchestrationVisitor<'_> {
    fn visit_expr(&mut self, expr: &'ast ast::Expr) {
        if let ast::ExprKind::MethodCall(method_call) = &expr.kind {
            if method_call.receiver.is_self_receiver() {
                let called = method_call.seg.ident.name.to_string();
                if self.pub_methods.contains(&called) {
                    self.offenders.push((called, method_call.seg.ident.span));
                }
            }
        }
        visit::walk_expr(self, expr);
    }
}

impl EarlyLintPass for NoSelfOrchestration {
    fn check_crate(&mut self, early_context: &EarlyContext<'_>, crate_root: &ast::Crate) {
        let mut collector = CollectVisitor {
            pub_methods_by_type: HashMap::new(),
        };
        visit::walk_crate(&mut collector, crate_root);

        let mut checker = CheckVisitor {
            early_context,
            pub_methods_by_type: &collector.pub_methods_by_type,
        };
        visit::walk_crate(&mut checker, crate_root);
    }
}
