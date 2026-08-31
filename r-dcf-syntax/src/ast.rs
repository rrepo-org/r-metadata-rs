//! Typed transient AST wrappers.

use std::fmt;

use crate::{SourceSpan, SyntaxKind, SyntaxNode};

/// Typed document root.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Root {
    syntax: SyntaxNode,
}

impl Root {
    pub(crate) fn new(syntax: SyntaxNode) -> Self {
        debug_assert_eq!(syntax.kind(), SyntaxKind::Root);
        Self { syntax }
    }

    /// Returns the underlying Rowan node.
    pub fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }

    /// Iterates over records in source order.
    pub fn records(&self) -> impl Iterator<Item = Record> + '_ {
        self.syntax.children().filter_map(Record::cast)
    }

    /// Returns the complete source range.
    pub fn source_range(&self) -> SourceSpan {
        SourceSpan::from_text_range(self.syntax.text_range())
    }
}

impl fmt::Display for Root {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.syntax, formatter)
    }
}

/// Typed DCF record.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Record {
    syntax: SyntaxNode,
}

impl Record {
    pub(crate) fn cast(syntax: SyntaxNode) -> Option<Self> {
        (syntax.kind() == SyntaxKind::Record).then_some(Self { syntax })
    }

    /// Returns the underlying Rowan node.
    pub fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }

    /// Iterates over every valid field, including duplicates.
    pub fn fields(&self) -> impl Iterator<Item = Field> + '_ {
        self.syntax.children().filter_map(Field::cast)
    }

    /// Iterates over fields whose names exactly equal `name`.
    pub fn fields_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = Field> + 'a {
        self.fields()
            .filter(move |field| field.name().as_deref() == Some(name))
    }

    /// Returns the first field whose name exactly equals `name`.
    pub fn field(&self, name: &str) -> Option<Field> {
        self.fields_named(name).next()
    }

    /// Returns the last field whose name exactly equals `name`.
    pub fn last_field(&self, name: &str) -> Option<Field> {
        self.fields_named(name).last()
    }

    /// Returns the record's source range.
    pub fn source_range(&self) -> SourceSpan {
        SourceSpan::from_text_range(self.syntax.text_range())
    }

    /// Returns the exact record text.
    pub fn raw_text(&self) -> String {
        self.syntax.to_string()
    }
}

impl fmt::Display for Record {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.syntax, formatter)
    }
}

/// Typed valid DCF field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
    syntax: SyntaxNode,
}

impl Field {
    pub(crate) fn cast(syntax: SyntaxNode) -> Option<Self> {
        (syntax.kind() == SyntaxKind::Field).then_some(Self { syntax })
    }

    /// Returns the underlying Rowan node.
    pub fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }

    /// Returns the case-sensitive field name.
    pub fn name(&self) -> Option<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .find(|token| token.kind() == SyntaxKind::Name)
            .map(|token| token.text().to_owned())
    }

    /// Returns the unfolded logical value and its enclosing source range.
    pub fn value(&self) -> ValueText {
        let raw = self.raw_text();
        let tokens: Vec<_> = self
            .syntax
            .children_with_tokens()
            .filter_map(rowan::NodeOrToken::into_token)
            .collect();
        let value_start = tokens
            .iter()
            .find(|token| token.kind() == SyntaxKind::Colon)
            .map_or(self.syntax.text_range().start(), |token| {
                token.text_range().end()
            });
        let value_end = tokens
            .iter()
            .rev()
            .find(|token| token.kind() != SyntaxKind::LineEnding)
            .map_or(value_start, |token| token.text_range().end());
        let range = SourceSpan::from_text_range(rowan::TextRange::new(value_start, value_end));
        let mut logical = Vec::new();
        for (index, line) in split_bodies(&raw).into_iter().enumerate() {
            let content = if index == 0 {
                line.split_once(':').map_or("", |(_, value)| value)
            } else {
                line.trim_start_matches([' ', '\t'])
            };
            let content = content.trim_matches([' ', '\t']);
            logical.push(if index > 0 && content == "." {
                ""
            } else {
                content
            });
        }
        ValueText {
            text: logical.join("\n"),
            range,
        }
    }

    /// Returns the field's source range, including its final line ending.
    pub fn source_range(&self) -> SourceSpan {
        SourceSpan::from_text_range(self.syntax.text_range())
    }

    /// Returns the exact physical field text.
    pub fn raw_text(&self) -> String {
        self.syntax.to_string()
    }
}

impl fmt::Display for Field {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.syntax, formatter)
    }
}

/// Owned unfolded field text and the enclosing physical source range.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValueText {
    text: String,
    range: SourceSpan,
}

impl ValueText {
    /// Returns the logical unfolded text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns the contiguous physical value range.
    ///
    /// For continued values this includes intervening line endings and
    /// indentation, but excludes the field name, colon, and final line ending.
    pub const fn source_range(&self) -> SourceSpan {
        self.range
    }

    /// Consumes this value and returns its text.
    pub fn into_string(self) -> String {
        self.text
    }
}

impl fmt::Display for ValueText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

fn split_bodies(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut result = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while cursor < bytes.len() {
        if matches!(bytes[cursor], b'\r' | b'\n') {
            result.push(&text[start..cursor]);
            if bytes[cursor] == b'\r' && bytes.get(cursor + 1) == Some(&b'\n') {
                cursor += 1;
            }
            cursor += 1;
            start = cursor;
        } else {
            cursor += 1;
        }
    }
    if start < text.len() {
        result.push(&text[start..]);
    }
    result
}
