use std::path::Path;

#[test]
fn ui() {
    let ui_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), ui_dir);
}

#[test]
fn ui_examples_thiserror_naming() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "thiserror_naming");
}
