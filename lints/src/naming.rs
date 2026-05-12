use std::collections::HashMap;

use rustc_ast::ast;
use rustc_ast::visit::{self, FnKind, Visitor};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

declare_lint! {
    /// **Deny** — every binding's name must derive from its type's `snake_case`
    /// name. Function parameters and `let` bindings with an explicit type
    /// ascription are checked. For generic-typed bindings the rule resolves
    /// the generic's bounds: with one effective bound the binding must match
    /// the trait name; with multiple bounds the generic itself must be given
    /// a descriptive identifier.
    pub TYPE_DERIVED_NAMING,
    Deny,
    "binding name must be snake_case of its declared type"
}

pub struct TypeDerivedNaming;

impl_lint_pass!(TypeDerivedNaming => [TYPE_DERIVED_NAMING]);

#[allow(raw_primitive_param)]
fn snake_case(s: &str) -> String {
    s.chars()
        .fold(
            (String::with_capacity(s.len() + 4), false),
            |(mut out, prev_lower), c| match c.is_ascii_uppercase() {
                false => {
                    out.push(c);
                    let new_prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
                    (out, new_prev_lower)
                },
                true => {
                    if prev_lower {
                        out.push('_');
                    }
                    out.extend(c.to_lowercase());
                    (out, false)
                },
            },
        )
        .0
}

fn extract_type_simple_name(ty: &ast::Ty) -> Option<String> {
    match &ty.kind {
        ast::TyKind::Path(_, path) => path.segments.last().map(|s| s.ident.name.to_string()),
        ast::TyKind::Ref(_, mut_ty) => extract_type_simple_name(&mut_ty.ty),
        _ => None,
    }
}

// WHY: auto traits and the default `Sized` bound carry no semantic
// "what is this thing" info; they describe capabilities the type system
// would otherwise infer or impose by default. Filtering them lets
// `<M: Migrator + Send>` count as a single-bound generic.
#[allow(raw_primitive_param)]
fn is_auto_or_default_bound(name: &str) -> bool {
    matches!(name, "?Sized" | "Send" | "Sized" | "Sync" | "Unpin")
}

// WHY: a single uppercase letter signals a placeholder generic parameter
// (Rust convention: `T`, `K`, `V`, `E`, `M`). When such a generic carries
// multiple trait bounds, demanding the binding match any single trait makes
// no sense — the right fix is to rename the generic to its role.
#[allow(raw_primitive_param)]
fn is_placeholder_generic_name(name: &str) -> bool {
    name.chars().count() == 1 && name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

#[allow(raw_primitive_param)]
fn is_primitive_type_name(name: &str) -> bool {
    matches!(
        name,
        "String"
            | "bool"
            | "char"
            | "f32"
            | "f64"
            | "i128"
            | "i16"
            | "i32"
            | "i64"
            | "i8"
            | "isize"
            | "str"
            | "u128"
            | "u16"
            | "u32"
            | "u64"
            | "u8"
            | "usize"
    )
}

#[allow(raw_primitive_param)]
fn is_stdlib_container(name: &str) -> bool {
    matches!(
        name,
        "Arc"
            | "BTreeMap"
            | "BTreeSet"
            | "Box"
            | "Cell"
            | "Cow"
            | "HashMap"
            | "HashSet"
            | "Mutex"
            | "Option"
            | "Path"
            | "PathBuf"
            | "PhantomData"
            | "Rc"
            | "RefCell"
            | "Result"
            | "RwLock"
            | "Vec"
    )
}

#[allow(raw_primitive_param)]
fn matches_expected(binding_name: &str, expected: &str) -> bool {
    binding_name == expected
        || binding_name.ends_with(&format!("_{expected}"))
        || binding_name.starts_with(&format!("{expected}_"))
}

fn trait_bound_names(bounds: &[ast::GenericBound]) -> Vec<String> {
    bounds
        .iter()
        .filter_map(|bound| match bound {
            ast::GenericBound::Outlives(_) => None,
            ast::GenericBound::Trait(poly_trait_ref) => {
                let name = poly_trait_ref
                    .trait_ref
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident.name.to_string())?;
                let prefix = match poly_trait_ref.modifiers.polarity {
                    ast::BoundPolarity::Maybe(_) => "?",
                    _ => "",
                };
                Some(format!("{prefix}{name}"))
            },
            ast::GenericBound::Use(..) => None,
        })
        .collect()
}

fn generic_param_bounds(generics: &ast::Generics) -> HashMap<String, Vec<String>> {
    generics
        .params
        .iter()
        .filter_map(|param| match &param.kind {
            ast::GenericParamKind::Type { .. } => Some((
                param.ident.name.to_string(),
                trait_bound_names(&param.bounds),
            )),
            _ => None,
        })
        .collect()
}

struct BindingName<'a>(&'a str);

struct Binding<'a> {
    binding_name: BindingName<'a>,
    span: Span,
    ty: &'a ast::Ty,
}

struct NamingVisitor<'cx> {
    early_context: &'cx EarlyContext<'cx>,
    scopes: Vec<HashMap<String, Vec<String>>>,
}

impl NamingVisitor<'_> {
    fn check_binding(&self, binding: Binding<'_>) {
        let Binding {
            binding_name: BindingName(binding_name),
            span,
            ty,
        } = binding;
        if binding_name.starts_with('_') {
            return;
        }
        let Some(type_name) = extract_type_simple_name(ty) else {
            return;
        };

        if let Some(bounds) = self.lookup_generic(&type_name) {
            let effective: Vec<&str> = bounds
                .iter()
                .filter(|b| !is_auto_or_default_bound(b))
                .map(String::as_str)
                .collect();
            match effective.as_slice() {
                [] => return,
                [single_bound] => {
                    let expected = snake_case(single_bound);
                    if matches_expected(binding_name, &expected) {
                        return;
                    }
                    self.early_context.opt_span_lint(
                        TYPE_DERIVED_NAMING,
                        Some(span),
                        |diag| {
                            diag.primary_message(format!(
                                "binding `{binding_name}` should be named `{expected}` (or `<prefix>_{expected}` / `{expected}_<suffix>`) to match its trait bound `{single_bound}`"
                            ));
                        },
                    );
                    return;
                },
                _ => {
                    if is_placeholder_generic_name(&type_name) {
                        let bounds_str = effective.join(" + ");
                        self.early_context.opt_span_lint(
                            TYPE_DERIVED_NAMING,
                            Some(span),
                            |diag| {
                                diag.primary_message(format!(
                                    "generic `{type_name}` has multiple trait bounds (`{bounds_str}`) — rename the generic to a descriptive identifier reflecting its role, then name `{binding_name}` after it"
                                ));
                            },
                        );
                        return;
                    }
                    let expected = snake_case(&type_name);
                    if matches_expected(binding_name, &expected) {
                        return;
                    }
                    self.early_context.opt_span_lint(
                        TYPE_DERIVED_NAMING,
                        Some(span),
                        |diag| {
                            diag.primary_message(format!(
                                "binding `{binding_name}` should be named `{expected}` (or `<prefix>_{expected}` / `{expected}_<suffix>`) to match generic `{type_name}`"
                            ));
                        },
                    );
                    return;
                },
            }
        }

        if is_primitive_type_name(&type_name) || is_stdlib_container(&type_name) {
            return;
        }
        let expected = snake_case(&type_name);
        if matches_expected(binding_name, &expected) {
            return;
        }
        self.early_context
            .opt_span_lint(TYPE_DERIVED_NAMING, Some(span), |diag| {
                diag.primary_message(format!(
                    "binding `{binding_name}` should be named `{expected}` (or `<prefix>_{expected}` / `{expected}_<suffix>`) to match type `{type_name}`"
                ));
            });
    }

    #[allow(raw_primitive_param)]
    fn lookup_generic(&self, name: &str) -> Option<&Vec<String>> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }
}

impl<'ast> Visitor<'ast> for NamingVisitor<'_> {
    fn visit_fn(
        &mut self,
        fn_kind: FnKind<'ast>,
        _attrs: &ast::AttrVec,
        _span: Span,
        _id: ast::NodeId,
    ) {
        if let FnKind::Fn(_, _, fn_box) = fn_kind {
            self.scopes.push(generic_param_bounds(&fn_box.generics));
            visit::walk_fn(self, fn_kind);
            self.scopes.pop();
        } else {
            visit::walk_fn(self, fn_kind);
        }
    }

    fn visit_item(&mut self, item: &'ast ast::Item) {
        if let ast::ItemKind::Impl(ref impl_block) = item.kind {
            self.scopes.push(generic_param_bounds(&impl_block.generics));
            visit::walk_item(self, item);
            self.scopes.pop();
        } else {
            visit::walk_item(self, item);
        }
    }

    fn visit_local(&mut self, local: &'ast ast::Local) {
        if !local.span.from_expansion() {
            if let Some(ty) = &local.ty {
                if let ast::PatKind::Ident(_, ident, _) = &local.pat.kind {
                    self.check_binding(Binding {
                        binding_name: BindingName(ident.name.as_str()),
                        span: local.pat.span,
                        ty,
                    });
                }
            }
        }
        visit::walk_local(self, local);
    }

    fn visit_param(&mut self, param: &'ast ast::Param) {
        if !param.span.from_expansion() && !param.is_self() {
            if let ast::PatKind::Ident(_, ident, _) = &param.pat.kind {
                self.check_binding(Binding {
                    binding_name: BindingName(ident.name.as_str()),
                    span: param.pat.span,
                    ty: &param.ty,
                });
            }
        }
        visit::walk_param(self, param);
    }
}

impl EarlyLintPass for TypeDerivedNaming {
    fn check_crate(&mut self, early_context: &EarlyContext<'_>, crate_root: &ast::Crate) {
        let mut visitor = NamingVisitor {
            early_context,
            scopes: Vec::new(),
        };
        visit::walk_crate(&mut visitor, crate_root);
    }
}
