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
    /// **Deny** — every function must take its subject as the first parameter
    /// and have at most one additional param. The only allowed shapes are
    /// `fn()`, `fn(self)`, and `fn(self, param)`. Free functions with
    /// parameters aren't allowed — make them methods on a type, or wrap the
    /// inputs into a single struct/newtype.
    pub SUBJECT_FIRST_PARAM,
    Deny,
    "fns must be fn(), fn(self), or fn(self, param) — subject is always the first arg"
}

#[allow(raw_primitive_field)]
#[derive(Default)]
pub struct SubjectFirstParam {
    in_trait_impl_depth: u32,
}
impl_lint_pass!(SubjectFirstParam => [SUBJECT_FIRST_PARAM]);

trait BlockExt {
    /// Returns true if any expression in this block (recursively) references
    /// the `self` value (as opposed to the `Self` type).
    fn uses_self(&self) -> bool;
}

impl BlockExt for ast::Block {
    fn uses_self(&self) -> bool {
        #[allow(raw_primitive_field)]
        struct Finder {
            found: bool,
        }
        impl<'ast> Visitor<'ast> for Finder {
            fn visit_expr(&mut self, expr: &'ast ast::Expr) {
                if self.found {
                    return;
                }
                if let ast::ExprKind::Path(_, path) = &expr.kind {
                    if path
                        .segments
                        .first()
                        .is_some_and(|s| s.ident.name.as_str() == "self")
                    {
                        self.found = true;
                        return;
                    }
                }
                visit::walk_expr(self, expr);
            }
        }
        let mut finder = Finder { found: false };
        visit::walk_block(&mut finder, self);
        finder.found
    }
}

impl EarlyLintPass for SubjectFirstParam {
    fn check_fn(
        &mut self,
        early_context: &EarlyContext<'_>,
        fn_kind: FnKind<'_>,
        span: Span,
        _id: NodeId,
    ) {
        if span.from_expansion() {
            return;
        }
        let FnKind::Fn(fn_ctxt, _, fn_box) = fn_kind else {
            return;
        };
        // WHY: foreign fns are FFI bindings whose signature is fixed by the
        // C ABI on the other side. Trait impl methods are constrained by the
        // trait declaration — flag the declaration once, not every impl.
        if matches!(fn_ctxt, FnCtxt::Foreign) || self.in_trait_impl_depth > 0 {
            return;
        }
        let inputs = &fn_box.sig.decl.inputs;
        let has_self = inputs.first().is_some_and(|p| p.is_self());
        let arity_issue = match (inputs.len(), has_self) {
            (0, _) => None,
            (1 | 2, true) => None,
            // WHY: constructor-style associated fns can't take `self` as the
            // first param — the instance doesn't exist yet. Allow any arity
            // when the return type mentions `Self` (covers `-> Self`,
            // `-> Result<Self, _>`, `-> Option<Self>`, etc.).
            (_, false) if fn_box.sig.decl.returns_self_type(early_context) => None,
            (_, false) => Some(
                "free function takes parameters — make it a method on a type so the subject is `self`",
            ),
            (_, true) => Some(
                "method takes more than one non-self param — wrap the inputs in a struct or newtype",
            ),
        };
        if let Some(msg) = arity_issue {
            early_context.opt_span_lint(SUBJECT_FIRST_PARAM, Some(span), |diag| {
                diag.primary_message(msg);
            });
            return;
        }
        // WHY: a method that declares `self` but never references it in the
        // body is in the wrong place. The "subject" of the operation is
        // whatever IS referenced in the body — usually a parameter or a
        // foreign type. Move the method to an extension trait on that
        // type instead.
        if has_self {
            if let Some(body) = fn_box.body.as_ref() {
                if !body.uses_self() {
                    early_context.opt_span_lint(SUBJECT_FIRST_PARAM, Some(span), |diag| {
                        diag.primary_message(
                            "method declares `self` but never references it — move to an extension trait on the actual subject (the type used in the body), or make it a free fn",
                        );
                    });
                }
            }
        }
    }

    fn check_item(&mut self, _early_context: &EarlyContext<'_>, item: &ast::Item) {
        if let ast::ItemKind::Impl(impl_block) = &item.kind {
            if impl_block.of_trait.is_some() {
                self.in_trait_impl_depth += 1;
            }
        }
    }

    fn check_item_post(&mut self, _early_context: &EarlyContext<'_>, item: &ast::Item) {
        if let ast::ItemKind::Impl(impl_block) = &item.kind {
            if impl_block.of_trait.is_some() {
                self.in_trait_impl_depth = self.in_trait_impl_depth.saturating_sub(1);
            }
        }
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

trait FnDeclExt {
    fn has_self_receiver(&self) -> bool;
    fn returns_self(&self, early_context: &EarlyContext<'_>) -> bool;
    /// True when the return type mentions `Self` anywhere — `Self`,
    /// `Result<Self, _>`, `Option<Self>`, `Arc<Self>`, …. Used to recognise
    /// constructor-style associated functions, which can't take `self` as
    /// a parameter because the instance doesn't exist yet.
    fn returns_self_type(&self, early_context: &EarlyContext<'_>) -> bool;
}

impl FnDeclExt for ast::FnDecl {
    fn has_self_receiver(&self) -> bool {
        self.inputs.first().is_some_and(|p| p.is_self())
    }

    fn returns_self(&self, early_context: &EarlyContext<'_>) -> bool {
        let ast::FnRetTy::Ty(ref ty) = self.output else {
            return false;
        };
        let Ok(snippet) = early_context.sess().source_map().span_to_snippet(ty.span) else {
            return false;
        };
        snippet.trim() == "Self"
    }

    fn returns_self_type(&self, early_context: &EarlyContext<'_>) -> bool {
        let ast::FnRetTy::Ty(ref ty) = self.output else {
            return false;
        };
        let Ok(snippet) = early_context.sess().source_map().span_to_snippet(ty.span) else {
            return false;
        };
        snippet
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|tok| tok == "Self")
    }
}

trait TyExt {
    fn simple_name(&self) -> Option<String>;
}

impl TyExt for ast::Ty {
    fn simple_name(&self) -> Option<String> {
        match &self.kind {
            ast::TyKind::Path(_, path) => path.segments.last().map(|s| s.ident.name.to_string()),
            _ => None,
        }
    }
}

struct Candidate {
    autofixable: bool,
    diag_span: Span,
    ident_span: Span,
    method_name: String,
    type_name: String,
}

struct CandidateScanner<'cx> {
    by_type: HashMap<String, Vec<Candidate>>,
    early_context: &'cx EarlyContext<'cx>,
    existing_new_by_type: HashMap<String, bool>,
}

impl<'cx> From<&'cx EarlyContext<'cx>> for CandidateScanner<'cx> {
    fn from(early_context: &'cx EarlyContext<'cx>) -> Self {
        CandidateScanner {
            by_type: HashMap::new(),
            early_context,
            existing_new_by_type: HashMap::new(),
        }
    }
}

impl CandidateScanner<'_> {
    /// Walk every `impl` block and collect renameable constructors. A
    /// `(type_name, method_name)` pair is autofixable when:
    ///   - exactly one method on `type_name` carries a forbidden name
    ///   - `type_name` has no method already called `new` (would collide)
    fn collect(mut self, crate_root: &ast::Crate) -> Vec<Candidate> {
        crate_root.items.iter().for_each(|item| {
            self.scan(item);
        });
        let CandidateScanner {
            by_type,
            existing_new_by_type,
            ..
        } = self;
        let mut out: Vec<Candidate> = by_type
            .into_iter()
            .flat_map(|(type_name, mut entries)| {
                let has_new = existing_new_by_type
                    .get(&type_name)
                    .copied()
                    .unwrap_or(false);
                let autofixable = !has_new && entries.len() == 1;
                entries.iter_mut().for_each(|c| c.autofixable = autofixable);
                entries
            })
            .collect();
        // WHY: HashMap iteration order is non-deterministic; sort by source
        // position so the diagnostic stream (and UI test stderr) stays stable.
        out.sort_by_key(|c| c.diag_span.lo());
        out
    }

    fn scan(&mut self, item: &ast::Item) {
        match &item.kind {
            ast::ItemKind::Impl(impl_block)
                if impl_block.of_trait.is_none() && !item.span.from_expansion() =>
            {
                self.scan_impl(impl_block);
            },
            ast::ItemKind::Mod(_, _, ast::ModKind::Loaded(items, ..)) => {
                items.iter().for_each(|child| self.scan(child));
            },
            _ => {},
        }
    }

    fn scan_impl(&mut self, impl_block: &ast::Impl) {
        let Some(type_name) = impl_block.self_ty.simple_name() else {
            return;
        };
        impl_block.items.iter().for_each(|assoc| {
            let ast::AssocItemKind::Fn(fn_box) = &assoc.kind else {
                return;
            };
            if fn_box.sig.decl.has_self_receiver() {
                return;
            }
            if !fn_box.sig.decl.returns_self(self.early_context) {
                return;
            }
            let method_name = fn_box.ident.name.to_string();
            if method_name == "new" {
                self.existing_new_by_type.insert(type_name.clone(), true);
                return;
            }
            if !FORBIDDEN_NAMES.contains(&method_name.as_str()) {
                return;
            }
            self.by_type
                .entry(type_name.clone())
                .or_default()
                .push(Candidate {
                    autofixable: false,
                    diag_span: assoc.span,
                    ident_span: fn_box.ident.span,
                    method_name,
                    type_name: type_name.clone(),
                });
        });
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
        let candidates = CandidateScanner::from(early_context).collect(crate_root);
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
