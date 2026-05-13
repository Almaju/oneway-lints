use rustc_ast::ast;
use rustc_ast::visit::{FnCtxt, FnKind};
use rustc_ast::NodeId;
use rustc_errors::Applicability;
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

// WHY: only direct primitives at the field/param level are autofixable.
// A `&str` field can't become `&MyType(str)` (unsized inner), and a `&u32`
// field requires extra ceremony at every call site; leave those to manual
// fixes.
fn direct_primitive(ty: &ast::Ty) -> Option<&'static str> {
    match &ty.kind {
        ast::TyKind::Path(None, path)
            if path.segments.len() == 1 && path.segments[0].args.is_none() =>
        {
            let name = path.segments[0].ident.name.as_str();
            PRIMITIVES.iter().copied().find(|&p| p == name)
        },
        _ => None,
    }
}

#[allow(raw_primitive_param)]
fn snake_to_pascal(s: &str) -> String {
    s.split('_')
        .filter(|seg| !seg.is_empty())
        .map(|seg| {
            let mut chars = seg.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect()
}

fn visibility_snippet(early_context: &EarlyContext<'_>, vis: &ast::Visibility) -> String {
    match vis.kind {
        ast::VisibilityKind::Inherited => String::new(),
        _ => early_context
            .sess()
            .source_map()
            .span_to_snippet(vis.span)
            .ok()
            .filter(|s| !s.is_empty())
            .map(|s| format!("{s} "))
            .unwrap_or_default(),
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
        vdata.fields().iter().for_each(|field| {
            let Some(name) = field.ident else {
                return;
            };
            let Some(primitive) = primitive_name(&field.ty) else {
                return;
            };
            let msg = format!(
                "field `{}` uses raw primitive `{primitive}` — wrap it in a newtype",
                name.name
            );
            let direct = direct_primitive(&field.ty);
            let newtype_name = snake_to_pascal(name.name.as_str());
            early_context.opt_span_lint(RAW_PRIMITIVE_FIELD, Some(field.ty.span), |diag| {
                diag.primary_message(msg);
                // WHY: skip autofix for refs (handled by `direct` filter) and
                // for degenerate field names that would produce an empty
                // newtype identifier (e.g. a field literally named `_`).
                let autofix_primitive = match newtype_name.is_empty() {
                    true => None,
                    false => direct,
                };
                if let Some(primitive) = autofix_primitive {
                    let vis = visibility_snippet(early_context, &field.vis);
                    // WHY: inner-field visibility matches outer so callers
                    // that already construct the parent struct can still
                    // construct the newtype literal at the same site.
                    let inner_vis = vis.clone();
                    let decl = format!("\n\n{vis}struct {newtype_name}({inner_vis}{primitive});");
                    // WHY: insert AFTER the parent struct's closing brace so
                    // any preceding `#[derive(...)]` keeps applying only to
                    // the parent — inserting BEFORE the parent would let the
                    // derive attach to the newtype instead.
                    let parts = vec![
                        (field.ty.span, newtype_name.clone()),
                        (item.span.shrink_to_hi(), decl),
                    ];
                    diag.multipart_suggestion(
                        "introduce a newtype (call sites will need to wrap their values)",
                        parts,
                        Applicability::MachineApplicable,
                    );
                }
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
        fn_span: Span,
        _id: NodeId,
    ) {
        let FnKind::Fn(fn_ctxt, _, fn_box) = fn_kind else {
            return;
        };
        if fn_box.body.is_none() {
            return;
        }
        // WHY: autofix only at free-fn level. Inside `impl`, inserting before
        // the fn span would put a struct declaration inside the impl block,
        // which isn't valid Rust. Same problem for trait methods. Foreign
        // functions (`extern { fn ... }`) can't even have bodies. Diagnostic
        // still fires; the human handles those cases.
        let autofix_allowed = matches!(fn_ctxt, FnCtxt::Free);
        fn_box
            .sig
            .decl
            .inputs
            .iter()
            .filter(|param| !param.span.from_expansion() && !param.is_self())
            .for_each(|param| {
                let Some(primitive) = primitive_name(&param.ty) else {
                    return;
                };
                let name = match &param.pat.kind {
                    ast::PatKind::Ident(_, ident, _) => ident.name.to_string(),
                    _ => "_".to_string(),
                };
                let msg = format!(
                    "param `{name}` uses raw primitive `{primitive}` — wrap it in a newtype"
                );
                let direct = direct_primitive(&param.ty);
                let newtype_name = snake_to_pascal(&name);
                early_context.opt_span_lint(RAW_PRIMITIVE_PARAM, Some(param.span), |diag| {
                    diag.primary_message(msg);
                    let autofix_primitive = match autofix_allowed && !newtype_name.is_empty() {
                        false => None,
                        true => direct,
                    };
                    if let Some(primitive) = autofix_primitive {
                        let decl = format!("struct {newtype_name}({primitive});\n\n");
                        let parts = vec![
                            (fn_span.shrink_to_lo(), decl),
                            (param.ty.span, newtype_name.clone()),
                        ];
                        diag.multipart_suggestion(
                            "introduce a newtype (body uses of the param and call sites will need updating)",
                            parts,
                            Applicability::MachineApplicable,
                        );
                    }
                });
            });
    }
}
