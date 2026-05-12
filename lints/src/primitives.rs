use rustc_ast::ast;
use rustc_ast::visit::FnKind;
use rustc_ast::NodeId;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

const PRIMITIVES: &[&str] = &[
    "String", "bool", "char", "f32", "f64", "i128", "i16", "i32", "i64", "i8", "isize", "str",
    "u128", "u16", "u32", "u64", "u8", "usize",
];

fn primitive_name(ty: &ast::Ty) -> Option<&'static str> {
    match &ty.kind {
        ast::TyKind::Path(None, path)
            if path.segments.len() == 1 && path.segments[0].args.is_none() =>
        {
            let name = path.segments[0].ident.name.as_str();
            PRIMITIVES.iter().copied().find(|&p| p == name)
        }
        ast::TyKind::Ref(_, mut_ty) => primitive_name(&mut_ty.ty),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// RAW_PRIMITIVE_FIELD
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Warn** — struct fields should use newtypes instead of raw primitives.
    pub RAW_PRIMITIVE_FIELD,
    Warn,
    "struct fields should wrap raw primitives in a newtype"
}

pub struct RawPrimitiveField;
impl_lint_pass!(RawPrimitiveField => [RAW_PRIMITIVE_FIELD]);

impl EarlyLintPass for RawPrimitiveField {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &ast::Item) {
        if item.span.from_expansion() {
            return;
        }
        let ast::ItemKind::Struct(_, _, ref vdata) = item.kind else {
            return;
        };
        for field in vdata.fields() {
            let Some(name) = field.ident else { continue };
            let Some(primitive) = primitive_name(&field.ty) else {
                continue;
            };
            cx.opt_span_lint(RAW_PRIMITIVE_FIELD, Some(field.ty.span), |diag| {
                diag.primary_message(format!(
                    "field `{}` uses raw primitive `{primitive}` — wrap it in a newtype",
                    name.name
                ));
            });
        }
    }
}

// ---------------------------------------------------------------------------
// RAW_PRIMITIVE_PARAM
// ---------------------------------------------------------------------------

declare_lint! {
    /// **Warn** — function parameters should use newtypes instead of raw
    /// primitives.
    pub RAW_PRIMITIVE_PARAM,
    Warn,
    "function parameters should wrap raw primitives in a newtype"
}

pub struct RawPrimitiveParam;
impl_lint_pass!(RawPrimitiveParam => [RAW_PRIMITIVE_PARAM]);

impl EarlyLintPass for RawPrimitiveParam {
    fn check_fn(&mut self, cx: &EarlyContext<'_>, kind: FnKind<'_>, _span: Span, _id: NodeId) {
        let FnKind::Fn(_, _, fn_box) = kind else {
            return;
        };
        if fn_box.body.is_none() {
            return;
        }
        for param in &fn_box.sig.decl.inputs {
            if param.span.from_expansion() || param.is_self() {
                continue;
            }
            let Some(primitive) = primitive_name(&param.ty) else {
                continue;
            };
            let name = match &param.pat.kind {
                ast::PatKind::Ident(_, ident, _) => ident.name.to_string(),
                _ => "_".to_string(),
            };
            cx.opt_span_lint(RAW_PRIMITIVE_PARAM, Some(param.span), |diag| {
                diag.primary_message(format!(
                    "param `{name}` uses raw primitive `{primitive}` — wrap it in a newtype"
                ));
            });
        }
    }
}
