//! Lossless syntax trees, builders, and text-preserving edits for R DCF files.
//!
//! Parsing is infallible: every input byte is represented in the syntax tree,
//! while malformed constructs are reported separately as diagnostics.
//!
//! # R DCF dialect
//!
//! R package metadata uses a dialect derived from Debian Control Files. It is
//! not interchangeable with Debian's deb822 application rules: R field names
//! are matched case-sensitively, duplicate declarations must remain available
//! to callers, and `#` lines do not receive `debian/control` comment semantics.
//! This crate also validates the portable field-name grammar used by R and
//! unfolds dot-only continuation lines as logical blank lines.
//!
//! These differences are why this crate has its own syntax model rather than
//! exposing [`deb822-lossless`](https://docs.rs/deb822-lossless) as the raw
//! layer. The parser retains malformed text in Rowan error nodes, preserves
//! physical whitespace and line endings, and offers exact first/last duplicate
//! lookup for the higher-level R metadata crates.

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
