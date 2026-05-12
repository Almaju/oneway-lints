use rustc_ast::ast;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

declare_lint! {
    /// **Deny** — every binding's name must be the `snake_case` version of its
    /// type. Applies to function parameters and `let` bindings with an
    /// explicit type ascription. Bindings of primitive types are exempt.
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

struct NameCheck<'a> {
    binding_name: &'a str,
    early_context: &'a EarlyContext<'a>,
    span: Span,
    ty: &'a ast::Ty,
}

fn check_name(name_check: NameCheck<'_>) {
    let NameCheck {
        binding_name,
        early_context,
        span,
        ty,
    } = name_check;
    if binding_name.starts_with('_') {
        return;
    }
    let Some(type_name) = extract_type_simple_name(ty) else {
        return;
    };
    if is_primitive_type_name(&type_name) || is_stdlib_container(&type_name) {
        return;
    }
    let expected = snake_case(&type_name);
    if binding_name == expected {
        return;
    }
    if binding_name.ends_with(&format!("_{expected}"))
        || binding_name.starts_with(&format!("{expected}_"))
    {
        return;
    }
    early_context.opt_span_lint(TYPE_DERIVED_NAMING, Some(span), |diag| {
        diag.primary_message(format!(
            "binding `{binding_name}` should be named `{expected}` (or `<prefix>_{expected}` / `{expected}_<suffix>`) to match type `{type_name}`"
        ));
    });
}

impl EarlyLintPass for TypeDerivedNaming {
    fn check_local(&mut self, early_context: &EarlyContext<'_>, local: &ast::Local) {
        if local.span.from_expansion() {
            return;
        }
        let Some(ty) = &local.ty else {
            return;
        };
        let ast::PatKind::Ident(_, ident, _) = &local.pat.kind else {
            return;
        };
        check_name(NameCheck {
            binding_name: ident.name.as_str(),
            early_context,
            span: local.pat.span,
            ty,
        });
    }

    fn check_param(&mut self, early_context: &EarlyContext<'_>, param: &ast::Param) {
        if param.span.from_expansion() || param.is_self() {
            return;
        }
        let ast::PatKind::Ident(_, ident, _) = &param.pat.kind else {
            return;
        };
        check_name(NameCheck {
            binding_name: ident.name.as_str(),
            early_context,
            span: param.pat.span,
            ty: &param.ty,
        });
    }
}
