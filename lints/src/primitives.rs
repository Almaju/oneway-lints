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
        },
        ast::TyKind::Ref(_, mut_ty) => primitive_name(&mut_ty.ty),
        _ => None,
    }
}

declare_lint! {
    /// **Warn** — struct fields should use newtypes instead of raw primitives.
    pub RAW_PRIMITIVE_FIELD,
    Warn,
    "struct fields should wrap raw primitives in a newtype"
}

pub struct RawPrimitiveField;
impl_lint_pass!(RawPrimitiveField => [RAW_PRIMITIVE_FIELD]);

impl EarlyLintPass for RawPrimitiveField {
    fn check_item(&mut self, early_context: &EarlyContext<'_>, item: &ast::Item) {
        if item.span.from_expansion() {
            return;
        }
        let ast::ItemKind::Struct(_, _, ref vdata) = item.kind else {
            return;
        };
        vdata
            .fields()
            .iter()
            .filter_map(|field| {
                let name = field.ident?;
                let primitive = primitive_name(&field.ty)?;
                Some((name, primitive, field.ty.span))
            })
            .for_each(|(name, primitive, span)| {
                early_context.opt_span_lint(RAW_PRIMITIVE_FIELD, Some(span), |diag| {
                    diag.primary_message(format!(
                        "field `{}` uses raw primitive `{primitive}` — wrap it in a newtype",
                        name.name
                    ));
                });
            });
    }
}

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
    fn check_fn(
        &mut self,
        early_context: &EarlyContext<'_>,
        fn_kind: FnKind<'_>,
        _span: Span,
        _id: NodeId,
    ) {
        let FnKind::Fn(_, _, fn_box) = fn_kind else {
            return;
        };
        if fn_box.body.is_none() {
            return;
        }
        fn_box
            .sig
            .decl
            .inputs
            .iter()
            .filter(|param| !param.span.from_expansion() && !param.is_self())
            .filter_map(|param| {
                let primitive = primitive_name(&param.ty)?;
                let name = match &param.pat.kind {
                    ast::PatKind::Ident(_, ident, _) => ident.name.to_string(),
                    _ => "_".to_string(),
                };
                Some((name, primitive, param.span))
            })
            .for_each(|(name, primitive, span)| {
                early_context.opt_span_lint(RAW_PRIMITIVE_PARAM, Some(span), |diag| {
                    diag.primary_message(format!(
                        "param `{name}` uses raw primitive `{primitive}` — wrap it in a newtype"
                    ));
                });
            });
    }
}
