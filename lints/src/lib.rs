#![feature(rustc_private)]
#![allow(unused_extern_crates)]
// WHY: rustc's clippy stage doesn't know about the dylint-defined lints
// declared in this crate (no_loop, no_if_else, type_derived_naming, etc.),
// so `#[allow(no_loop)]` on a state-machine function trips an `unknown_lints`
// warning under clippy. The lint names ARE valid under the dylint stage
// where they're registered. Silence the cross-stage diagnostic crate-wide.
#![allow(unknown_lints)]

extern crate rustc_ast;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

dylint_linting::dylint_library!();

mod control_flow;
mod functions;
mod naming;
mod organization;
mod primitives;
mod sorting;
mod style;

#[doc(hidden)]
#[no_mangle]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[
        sorting::UNSORTED_STRUCT_FIELDS,
        sorting::UNSORTED_ENUM_VARIANTS,
        sorting::UNSORTED_MATCH_ARMS,
        sorting::MOD_AFTER_USE,
        sorting::UNSORTED_IMPL_METHODS,
        sorting::UNSORTED_DERIVES,
    ]);
    lint_store.register_early_pass(|| Box::new(sorting::UnsortedStructFields));
    lint_store.register_early_pass(|| Box::new(sorting::UnsortedEnumVariants));
    lint_store.register_early_pass(|| Box::new(sorting::UnsortedMatchArms));
    lint_store.register_early_pass(|| Box::new(sorting::ModAfterUse));
    lint_store.register_early_pass(|| Box::new(sorting::UnsortedImplMethods));
    lint_store.register_early_pass(|| Box::new(sorting::UnsortedDerives));

    lint_store.register_lints(&[
        functions::NO_NESTED_FUNCTIONS,
        functions::ONE_CONSTRUCTOR_NAME,
    ]);
    lint_store.register_early_pass(|| Box::new(functions::NoNestedFunctions));
    lint_store.register_early_pass(|| Box::new(functions::OneConstructorName));

    lint_store.register_lints(&[control_flow::NO_LOOP, control_flow::NO_IF_ELSE]);
    lint_store.register_early_pass(|| Box::new(control_flow::NoLoop));
    lint_store.register_early_pass(|| Box::new(control_flow::NoIfElse));

    lint_store.register_lints(&[style::NO_COMMENTS, style::NO_TURBOFISH]);
    lint_store.register_early_pass(|| Box::new(style::NoComments));
    lint_store.register_early_pass(|| Box::new(style::NoTurbofish));

    lint_store.register_lints(&[
        primitives::RAW_PRIMITIVE_FIELD,
        primitives::RAW_PRIMITIVE_PARAM,
    ]);
    lint_store.register_early_pass(|| Box::new(primitives::RawPrimitiveField));
    lint_store.register_early_pass(|| Box::new(primitives::RawPrimitiveParam));

    lint_store.register_lints(&[organization::ONE_PUBLIC_TYPE_PER_FILE]);
    lint_store.register_early_pass(|| Box::new(organization::OnePublicTypePerFile::default()));

    lint_store.register_lints(&[naming::TYPE_DERIVED_NAMING]);
    lint_store.register_early_pass(|| Box::new(naming::TypeDerivedNaming));
}
