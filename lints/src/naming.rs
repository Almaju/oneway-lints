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

trait StrExt {
    /// WHY: auto traits and the default `Sized` bound carry no semantic
    /// "what is this thing" info; they describe capabilities the type system
    /// would otherwise infer or impose by default. Filtering them lets
    /// `<M: Migrator + Send>` count as a single-bound generic.
    fn is_auto_or_default_bound(&self) -> bool;
    /// WHY: a single uppercase letter signals a placeholder generic parameter
    /// (Rust convention: `T`, `K`, `V`, `E`, `M`). When such a generic carries
    /// multiple trait bounds, demanding the binding match any single trait
    /// makes no sense — the right fix is to rename the generic to its role.
    fn is_placeholder_generic_name(&self) -> bool;
    fn is_primitive_type_name(&self) -> bool;
    fn is_stdlib_container(&self) -> bool;
    fn matches_expected(&self, expected: &str) -> bool;
    fn snake_case(&self) -> String;
}

impl StrExt for str {
    fn is_auto_or_default_bound(&self) -> bool {
        matches!(self, "?Sized" | "Send" | "Sized" | "Sync" | "Unpin")
    }

    fn is_placeholder_generic_name(&self) -> bool {
        self.chars().count() == 1 && self.chars().next().is_some_and(|c| c.is_ascii_uppercase())
    }

    fn is_primitive_type_name(&self) -> bool {
        matches!(
            self,
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

    fn is_stdlib_container(&self) -> bool {
        matches!(
            self,
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

    fn matches_expected(&self, expected: &str) -> bool {
        self == expected
            || self.ends_with(&format!("_{expected}"))
            || self.starts_with(&format!("{expected}_"))
    }

    fn snake_case(&self) -> String {
        self.chars()
            .fold(
                (String::with_capacity(self.len() + 4), false),
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
}

trait TyExt {
    fn simple_name(&self) -> Option<String>;
}

impl TyExt for ast::Ty {
    fn simple_name(&self) -> Option<String> {
        match &self.kind {
            ast::TyKind::Path(_, path) => path.segments.last().map(|s| s.ident.name.to_string()),
            ast::TyKind::Ref(_, mut_ty) => mut_ty.ty.simple_name(),
            _ => None,
        }
    }
}

trait BoundsExt {
    fn trait_names(&self) -> Vec<String>;
}

impl BoundsExt for [ast::GenericBound] {
    fn trait_names(&self) -> Vec<String> {
        self.iter()
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
}

trait GenericsExt {
    fn param_bounds(&self) -> HashMap<String, Vec<String>>;
}

impl GenericsExt for ast::Generics {
    fn param_bounds(&self) -> HashMap<String, Vec<String>> {
        self.params
            .iter()
            .filter_map(|param| match &param.kind {
                ast::GenericParamKind::Type { .. } => {
                    Some((param.ident.name.to_string(), param.bounds.trait_names()))
                },
                _ => None,
            })
            .collect()
    }
}

struct BindingName<'a>(&'a str);

struct Binding<'a> {
    binding_name: BindingName<'a>,
    rename_refs: Option<&'a [RefKind]>,
    span: Span,
    ty: &'a ast::Ty,
}

struct Rename<'a> {
    decl_span: Span,
    msg: String,
    new_name: String,
    refs: Option<&'a [RefKind]>,
}

struct FnContext {
    pat_bindings: HashMap<String, usize>,
    refs: HashMap<String, Vec<RefKind>>,
}

/// A discovered reference to a local binding that may need to be renamed
/// alongside the binding declaration.
enum RefKind {
    /// A normal value-position reference: `let _ = id;`
    Plain(Span),
    /// A struct-expression shorthand: `Self { source }` is sugar for
    /// `Self { source: source }`. The span covers only the single
    /// identifier in source text, but the rename must expand to
    /// `source: new_name` so the field name still resolves.
    Shorthand { field_name: String, span: Span },
}

/// Collects local-binding references — both plain single-segment Path
/// expressions and struct-expression shorthand fields — for later renaming.
struct ReferenceCollector<'a> {
    refs: &'a mut HashMap<String, Vec<RefKind>>,
}

impl<'ast> Visitor<'ast> for ReferenceCollector<'_> {
    fn visit_expr(&mut self, expr: &'ast ast::Expr) {
        if let ast::ExprKind::Struct(struct_expr) = &expr.kind {
            struct_expr
                .fields
                .iter()
                .for_each(|field| match field.is_shorthand {
                    false => self.visit_expr(&field.expr),
                    true => {
                        let name = field.ident.name.to_string();
                        self.refs
                            .entry(name.clone())
                            .or_default()
                            .push(RefKind::Shorthand {
                                field_name: name,
                                span: field.ident.span,
                            });
                    },
                });
            // NOTE: still walk into the `..base` rest expression if present.
            if let ast::StructRest::Base(base) = &struct_expr.rest {
                self.visit_expr(base);
            }
            return;
        }
        if let ast::ExprKind::Path(None, path) = &expr.kind {
            if path.segments.len() == 1 {
                let segment = &path.segments[0];
                if segment.args.is_none() {
                    let name = segment.ident.name.to_string();
                    self.refs
                        .entry(name)
                        .or_default()
                        .push(RefKind::Plain(segment.ident.span));
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

impl From<&ast::Block> for FnContext {
    fn from(block: &ast::Block) -> Self {
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
        let Some(type_name) = ty.simple_name() else {
            return;
        };

        if let Some(bounds) = self.lookup_generic(&type_name) {
            let effective: Vec<&str> = bounds
                .iter()
                .filter(|b| !b.is_auto_or_default_bound())
                .map(String::as_str)
                .collect();
            match effective.as_slice() {
                [] => return,
                [single_bound] => {
                    let expected = single_bound.snake_case();
                    if binding_name.matches_expected(&expected) {
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
                    if type_name.is_placeholder_generic_name() {
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
                    let expected = type_name.snake_case();
                    if binding_name.matches_expected(&expected) {
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

        if type_name.is_primitive_type_name() || type_name.is_stdlib_container() {
            return;
        }
        let expected = type_name.snake_case();
        if binding_name.matches_expected(&expected) {
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
                    refs.iter().for_each(|ref_kind| match ref_kind {
                        RefKind::Plain(span) => parts.push((*span, new_name.clone())),
                        RefKind::Shorthand { field_name, span } => {
                            parts.push((*span, format!("{field_name}: {new_name}")));
                        },
                    });
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
            self.scopes.push(fn_box.generics.param_bounds());
            let body_context = fn_box
                .body
                .as_ref()
                .map(|body| FnContext::from(body.as_ref()));
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
            self.scopes.push(impl_block.generics.param_bounds());
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
