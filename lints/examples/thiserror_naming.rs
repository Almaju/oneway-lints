// WHY: `thiserror`'s `#[derive(Error)]` macro generates an
// `impl From<Inner> for Outer { fn from(source: Inner) -> Self { ... } }`
// for each `#[from]` field. The generated `source` binding ident carries
// a span pointing back at the user's `#[from]` annotation with the user's
// `SyntaxContext` — so `param.span.from_expansion()` returns false and
// the lint used to fire on macro-generated bindings as if the user had
// hand-written them.
//
// This example exercises the exact pattern the issue report described.
// If `type_derived_naming` correctly recognises the binding as
// macro-generated, no diagnostic should fire.

use thiserror::Error;

pub struct DbError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DbError")
    }
}

impl std::fmt::Debug for DbError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DbError")
    }
}

impl std::error::Error for DbError {}

fn main() {}
