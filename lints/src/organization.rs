use std::collections::HashMap;

use rustc_ast::ast;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::{FileName, Span};

declare_lint! {
    /// **Warn** — each file should export at most one primary public type
    /// (named-field struct or enum). Tuple structs and unit structs are
    /// treated as supporting cast.
    pub ONE_PUBLIC_TYPE_PER_FILE,
    Warn,
    "each file should export at most one primary public type"
}

#[derive(Default)]
pub struct OnePublicTypePerFile {
    by_file: HashMap<String, Vec<Span>>,
}

impl_lint_pass!(OnePublicTypePerFile => [ONE_PUBLIC_TYPE_PER_FILE]);

fn is_primary_pub_type(item: &ast::Item) -> bool {
    let is_pub = matches!(
        item.vis.kind,
        ast::VisibilityKind::Public | ast::VisibilityKind::Restricted { .. }
    );
    if !is_pub {
        return false;
    }
    match &item.kind {
        ast::ItemKind::Enum(..) => true,
        ast::ItemKind::Struct(_, _, vdata) => {
            matches!(vdata, ast::VariantData::Struct { .. })
        },
        _ => false,
    }
}

fn is_local_source_path(path: &std::path::Path) -> bool {
    let s = path.to_string_lossy();
    !s.contains("/.cargo/")
        && !s.contains("/.rustup/")
        && !s.contains("/rustlib/")
        && !s.starts_with("<")
}

impl EarlyLintPass for OnePublicTypePerFile {
    fn check_crate_post(&mut self, early_context: &EarlyContext<'_>, _krate: &ast::Crate) {
        self.by_file
            .values()
            .filter(|spans| spans.len() > 1)
            .flat_map(|spans| spans.iter().skip(1))
            .for_each(|span| {
                early_context.opt_span_lint(ONE_PUBLIC_TYPE_PER_FILE, Some(*span), |diag| {
                    diag.primary_message(
                        "second primary public type in this file — extract into its own file",
                    );
                });
            });
    }

    fn check_item(&mut self, early_context: &EarlyContext<'_>, item: &ast::Item) {
        if item.span.from_expansion() || !is_primary_pub_type(item) {
            return;
        }
        let source_map = early_context.sess().source_map();
        let file = source_map.lookup_source_file(item.span.lo());
        let path = match &file.name {
            FileName::Real(real) => real.local_path_if_available().to_path_buf(),
            _ => return,
        };
        if !is_local_source_path(&path) {
            return;
        }
        let key = path.to_string_lossy().into_owned();
        self.by_file.entry(key).or_default().push(item.span);
    }
}
