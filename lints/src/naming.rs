use std::collections::HashMap;

use rustc_ast::ast;
use rustc_ast::visit::{self, FnKind, Visitor};
use rustc_errors::Applicability;
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
    rename_refs: Option<&'a [Span]>,
    span: Span,
    ty: &'a ast::Ty,
}

struct Rename<'a> {
    decl_span: Span,
    msg: String,
    new_name: String,
    refs: Option<&'a [Span]>,
}

struct FnContext {
    pat_bindings: HashMap<String, usize>,
    refs: HashMap<String, Vec<Span>>,
}

/// Collects single-segment Path expressions throughout a block — these are
/// the "value reference" sites that need to be renamed alongside the binding.
struct ReferenceCollector<'a> {
    refs: &'a mut HashMap<String, Vec<Span>>,
}

impl<'ast> Visitor<'ast> for ReferenceCollector<'_> {
    fn visit_expr(&mut self, expr: &'ast ast::Expr) {
        if let ast::ExprKind::Path(None, path) = &expr.kind {
            if path.segments.len() == 1 {
                let segment = &path.segments[0];
                if segment.args.is_none() {
                    let name = segment.ident.name.to_string();
                    self.refs.entry(name).or_default().push(segment.ident.span);
                }
            }
        }
        visit::walk_expr(self, expr);
    }
}

/// Counts how many times a name appears as a pattern binding inside a block.
/// Used to skip autofix when a param name is shadowed by an inner `let`,
/// `if let`, or match arm — renaming all refs would point past the shadow.
struct PatBindingCounter<'a> {
    counts: &'a mut HashMap<String, usize>,
}

impl<'ast> Visitor<'ast> for PatBindingCounter<'_> {
    fn visit_pat(&mut self, pat: &'ast ast::Pat) {
        if let ast::PatKind::Ident(_, ident, _) = &pat.kind {
            *self.counts.entry(ident.name.to_string()).or_default() += 1;
        }
        visit::walk_pat(self, pat);
    }
}

fn build_fn_context(block: &ast::Block) -> FnContext {
    let mut refs = HashMap::new();
    let mut collector = ReferenceCollector { refs: &mut refs };
    visit::walk_block(&mut collector, block);

    let mut pat_bindings = HashMap::new();
    let mut counter = PatBindingCounter {
        counts: &mut pat_bindings,
    };
    visit::walk_block(&mut counter, block);

    FnContext { pat_bindings, refs }
}

struct NamingVisitor<'cx> {
    early_context: &'cx EarlyContext<'cx>,
    fn_stack: Vec<FnContext>,
    scopes: Vec<HashMap<String, Vec<String>>>,
}

impl NamingVisitor<'_> {
    fn check_binding(&self, binding: Binding<'_>) {
        let Binding {
            binding_name: BindingName(binding_name),
            rename_refs,
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
                    let msg = format!(
                        "binding `{binding_name}` should be named `{expected}` (or `<prefix>_{expected}` / `{expected}_<suffix>`) to match its trait bound `{single_bound}`"
                    );
                    self.emit_rename(Rename {
                        decl_span: span,
                        msg,
                        new_name: expected,
                        refs: rename_refs,
                    });
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
                    let msg = format!(
                        "binding `{binding_name}` should be named `{expected}` (or `<prefix>_{expected}` / `{expected}_<suffix>`) to match generic `{type_name}`"
                    );
                    self.emit_rename(Rename {
                        decl_span: span,
                        msg,
                        new_name: expected,
                        refs: rename_refs,
                    });
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
        let msg = format!(
            "binding `{binding_name}` should be named `{expected}` (or `<prefix>_{expected}` / `{expected}_<suffix>`) to match type `{type_name}`"
        );
        self.emit_rename(Rename {
            decl_span: span,
            msg,
            new_name: expected,
            refs: rename_refs,
        });
    }

    fn emit_rename(&self, rename: Rename<'_>) {
        let Rename {
            decl_span,
            msg,
            new_name,
            refs,
        } = rename;
        self.early_context
            .opt_span_lint(TYPE_DERIVED_NAMING, Some(decl_span), |diag| {
                diag.primary_message(msg);
                if let Some(refs) = refs {
                    let mut parts: Vec<(Span, String)> = Vec::with_capacity(refs.len() + 1);
                    parts.push((decl_span, new_name.clone()));
                    refs.iter().for_each(|s| parts.push((*s, new_name.clone())));
                    diag.multipart_suggestion(
                        "rename the binding and all references",
                        parts,
                        Applicability::MachineApplicable,
                    );
                }
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
            let body_context = fn_box.body.as_ref().map(|body| build_fn_context(body));
            let pushed_fn = body_context.is_some();
            if let Some(ctx) = body_context {
                self.fn_stack.push(ctx);
            }
            visit::walk_fn(self, fn_kind);
            if pushed_fn {
                self.fn_stack.pop();
            }
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
                    // WHY: let-binding scope is the rest of the enclosing
                    // block (until shadowed), which we don't track precisely.
                    // Use fn-wide ref scope would over-rename across blocks;
                    // safer to emit diagnostic without autofix.
                    self.check_binding(Binding {
                        binding_name: BindingName(ident.name.as_str()),
                        rename_refs: None,
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
                let name = ident.name.as_str().to_string();
                // WHY: skip autofix if the name is shadowed in the body
                // by an inner `let`, `if let`, or match arm — those bring
                // a new binding into scope and renaming all references
                // would silently point past the shadow.
                let rename_refs = self.fn_stack.last().and_then(|ctx| {
                    match ctx.pat_bindings.get(&name).copied().unwrap_or(0) > 0 {
                        false => ctx.refs.get(&name).map(Vec::as_slice),
                        true => None,
                    }
                });
                self.check_binding(Binding {
                    binding_name: BindingName(&name),
                    rename_refs,
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
            fn_stack: Vec::new(),
            scopes: Vec::new(),
        };
        visit::walk_crate(&mut visitor, crate_root);
    }
}
