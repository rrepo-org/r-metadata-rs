//! Infallible lossless parser.

use std::{fmt, ops::Range, sync::Arc};

use rowan::{GreenNode, GreenNodeBuilder, Language};

use crate::{
    ast::{Record, Root},
    syntax::{DcfLanguage, SyntaxKind, SyntaxNode},
    value::is_valid_field_name,
};

/// A byte span in the UTF-8 source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourceSpan {
    /// Inclusive byte offset.
    pub start: usize,
    /// Exclusive byte offset.
    pub end: usize,
}

impl SourceSpan {
    /// Creates a span from byte offsets.
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns whether the span contains no bytes.
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Returns the number of bytes in the span.
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    pub(crate) fn from_text_range(range: rowan::TextRange) -> Self {
        Self::new(
            u32::from(range.start()) as usize,
            u32::from(range.end()) as usize,
        )
    }
}

impl From<SourceSpan> for Range<usize> {
    fn from(span: SourceSpan) -> Self {
        span.start..span.end
    }
}

/// Stable category assigned to a parse diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticKind {
    /// A non-indented line has no colon.
    MalformedField,
    /// A continuation line has no preceding field in its record.
    OrphanContinuation,
    /// Text before the first colon is not a valid field name.
    InvalidFieldName,
}

/// A parser diagnostic with a stable category and byte span.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    kind: DiagnosticKind,
    span: SourceSpan,
}

impl Diagnostic {
    /// Returns the stable diagnostic category.
    pub const fn kind(&self) -> DiagnosticKind {
        self.kind
    }

    /// Returns the offending source span.
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns a stable human-readable message.
    pub const fn message(&self) -> &'static str {
        match self.kind {
            DiagnosticKind::MalformedField => "field line is missing a colon",
            DiagnosticKind::OrphanContinuation => "continuation line has no preceding field",
            DiagnosticKind::InvalidFieldName => "invalid field name",
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at {}..{}",
            self.message(),
            self.span.start,
            self.span.end
        )
    }
}

/// Persistent parse result owning only a Rowan green tree and shared diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parse {
    green: GreenNode,
    diagnostics: Arc<[Diagnostic]>,
}

impl Parse {
    /// Parses UTF-8 R DCF text without failing.
    pub fn new(source: &str) -> Self {
        parse(source)
    }

    /// Returns the persistent immutable green tree.
    pub const fn green(&self) -> &GreenNode {
        &self.green
    }

    /// Materializes a transient typed syntax root.
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    /// Materializes the typed document root.
    pub fn root(&self) -> Root {
        Root::new(self.syntax())
    }

    /// Returns all diagnostics in source order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Clones the shared diagnostics allocation.
    pub fn diagnostics_arc(&self) -> Arc<[Diagnostic]> {
        Arc::clone(&self.diagnostics)
    }

    /// Iterates over records in source order.
    pub fn records(&self) -> impl Iterator<Item = Record> {
        self.root().records().collect::<Vec<_>>().into_iter()
    }
}

impl fmt::Display for Parse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.syntax(), formatter)
    }
}

/// Parses UTF-8 R DCF text into a lossless syntax tree.
///
/// # Panics
///
/// Panics when `source` exceeds Rowan's 4 GiB tree-size limit.
pub fn parse(source: &str) -> Parse {
    assert!(
        u32::try_from(source.len()).is_ok(),
        "Rowan input exceeds 4 GiB"
    );
    let lines = physical_lines(source);
    let mut builder = GreenNodeBuilder::new();
    let mut diagnostics = Vec::new();
    builder.start_node(DcfLanguage::kind_to_raw(SyntaxKind::Root));
    let mut record_open = false;
    let mut field_open = false;

    for line in lines {
        let body = &source[line.start..line.body_end];
        if body.bytes().all(|byte| matches!(byte, b' ' | b'\t' | 0x0c)) {
            close_field(&mut builder, &mut field_open);
            close_record(&mut builder, &mut record_open);
            token(&mut builder, SyntaxKind::Whitespace, body);
            token(
                &mut builder,
                SyntaxKind::LineEnding,
                &source[line.body_end..line.end],
            );
            continue;
        }

        if !record_open {
            builder.start_node(DcfLanguage::kind_to_raw(SyntaxKind::Record));
            record_open = true;
        }

        if body.starts_with([' ', '\t']) {
            if field_open {
                emit_continuation(&mut builder, source, line);
            } else {
                emit_error(&mut builder, source, line);
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::OrphanContinuation,
                    span: SourceSpan::new(line.start, line.body_end),
                });
            }
            continue;
        }

        close_field(&mut builder, &mut field_open);
        if let Some(colon) = body.find(':') {
            let name = &body[..colon];
            if is_valid_field_name(name) {
                builder.start_node(DcfLanguage::kind_to_raw(SyntaxKind::Field));
                field_open = true;
                token(&mut builder, SyntaxKind::Name, name);
                token(&mut builder, SyntaxKind::Colon, ":");
                token(&mut builder, SyntaxKind::Value, &body[colon + 1..]);
                token(
                    &mut builder,
                    SyntaxKind::LineEnding,
                    &source[line.body_end..line.end],
                );
            } else {
                emit_error(&mut builder, source, line);
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::InvalidFieldName,
                    span: SourceSpan::new(line.start, line.start + colon),
                });
            }
        } else {
            emit_error(&mut builder, source, line);
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::MalformedField,
                span: SourceSpan::new(line.start, line.body_end),
            });
        }
    }

    close_field(&mut builder, &mut field_open);
    close_record(&mut builder, &mut record_open);
    builder.finish_node();
    Parse {
        green: builder.finish(),
        diagnostics: diagnostics.into(),
    }
}

#[derive(Clone, Copy)]
struct PhysicalLine {
    start: usize,
    body_end: usize,
    end: usize,
}

fn physical_lines(source: &str) -> Vec<PhysicalLine> {
    let bytes = source.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while cursor < bytes.len() {
        if matches!(bytes[cursor], b'\r' | b'\n') {
            let body_end = cursor;
            cursor += 1;
            if bytes[body_end] == b'\r' && bytes.get(cursor) == Some(&b'\n') {
                cursor += 1;
            }
            lines.push(PhysicalLine {
                start,
                body_end,
                end: cursor,
            });
            start = cursor;
        } else {
            cursor += 1;
        }
    }
    if start < source.len() {
        lines.push(PhysicalLine {
            start,
            body_end: source.len(),
            end: source.len(),
        });
    }
    lines
}

fn emit_continuation(builder: &mut GreenNodeBuilder<'_>, source: &str, line: PhysicalLine) {
    let body = &source[line.start..line.body_end];
    let indent_len = body
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    token(builder, SyntaxKind::Indent, &body[..indent_len]);
    let content = &body[indent_len..];
    token(
        builder,
        if content == "." {
            SyntaxKind::Dot
        } else {
            SyntaxKind::Value
        },
        content,
    );
    token(
        builder,
        SyntaxKind::LineEnding,
        &source[line.body_end..line.end],
    );
}

fn emit_error(builder: &mut GreenNodeBuilder<'_>, source: &str, line: PhysicalLine) {
    builder.start_node(DcfLanguage::kind_to_raw(SyntaxKind::Error));
    token(
        builder,
        SyntaxKind::ErrorToken,
        &source[line.start..line.body_end],
    );
    token(
        builder,
        SyntaxKind::LineEnding,
        &source[line.body_end..line.end],
    );
    builder.finish_node();
}

fn token(builder: &mut GreenNodeBuilder<'_>, kind: SyntaxKind, text: &str) {
    if !text.is_empty() {
        builder.token(DcfLanguage::kind_to_raw(kind), text);
    }
}

fn close_field(builder: &mut GreenNodeBuilder<'_>, open: &mut bool) {
    if *open {
        builder.finish_node();
        *open = false;
    }
}

fn close_record(builder: &mut GreenNodeBuilder<'_>, open: &mut bool) {
    if *open {
        builder.finish_node();
        *open = false;
    }
}
