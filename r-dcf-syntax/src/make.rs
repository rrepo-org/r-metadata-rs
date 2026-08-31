//! Canonical text builders.

use crate::{FieldName, LogicalValue};

/// A physical line-ending convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LineEnding {
    /// Line feed (`\n`).
    #[default]
    Lf,
    /// Carriage return followed by line feed (`\r\n`).
    CrLf,
    /// A lone carriage return (`\r`).
    Cr,
}

impl LineEnding {
    /// Returns the physical delimiter.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }
}

/// Formatting choices used by canonical builders.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FormatStyle {
    /// Physical line ending.
    pub line_ending: LineEnding,
    /// Indentation for continuation lines.
    pub continuation_indent: String,
    /// Whether non-empty first lines receive one space after the colon.
    pub space_after_colon: bool,
}

impl Default for FormatStyle {
    fn default() -> Self {
        Self {
            line_ending: LineEnding::Lf,
            continuation_indent: " ".to_owned(),
            space_after_colon: true,
        }
    }
}

/// Builds one field without a trailing line ending.
pub fn field(name: &FieldName, value: &LogicalValue, style: &FormatStyle) -> String {
    let mut lines = value.as_str().split('\n');
    let first = lines.next().unwrap_or_default();
    let mut output = format!("{name}:");
    if style.space_after_colon && !first.is_empty() {
        output.push(' ');
    }
    output.push_str(first);
    for line in lines {
        output.push_str(style.line_ending.as_str());
        output.push_str(&style.continuation_indent);
        output.push_str(if line.is_empty() { "." } else { line });
    }
    output
}

/// Builds one record from already-built fields, without a trailing line ending.
pub fn record(fields: &[String], style: &FormatStyle) -> String {
    fields.join(style.line_ending.as_str())
}

/// Builds a document from already-built records, without a trailing line ending.
pub fn document(records: &[String], style: &FormatStyle) -> String {
    records.join(&format!("{0}{0}", style.line_ending.as_str()))
}
