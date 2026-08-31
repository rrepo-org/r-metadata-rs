//! Lossless syntax trees, builders, and text-preserving edits for R DCF files.
//!
//! Parsing is infallible: every input byte is represented in the syntax tree,
//! while malformed constructs are reported separately as diagnostics.

mod ast;
mod edit;
pub mod make;
mod parser;
mod syntax;
mod value;

#[cfg(test)]
mod tests;

pub use ast::{Field, Record, Root, ValueText};
pub use edit::{EditError, Editor};
pub use make::{FormatStyle, LineEnding, document, field, record};
pub use parser::{Diagnostic, DiagnosticKind, Parse, SourceSpan, parse};
pub use syntax::{DcfLanguage, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};
pub use value::{FieldName, InvalidFieldName, InvalidLogicalValue, LogicalValue};
