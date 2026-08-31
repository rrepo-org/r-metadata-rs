//! Rowan language and syntax kinds.

use rowan::Language;

/// The kinds of nodes and tokens in an R DCF syntax tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    /// The document root node.
    Root,
    /// A record node.
    Record,
    /// A field node.
    Field,
    /// A node containing malformed input.
    Error,
    /// A field-name token.
    Name,
    /// The first colon on a field line.
    Colon,
    /// Value text on a physical line.
    Value,
    /// Indentation at the start of a continuation line.
    Indent,
    /// A dot denoting a logical blank continuation line.
    Dot,
    /// Whitespace belonging to a record separator.
    Whitespace,
    /// An LF, CRLF, or lone CR line ending.
    LineEnding,
    /// Text inside an error node.
    ErrorToken,
}

/// Rowan language marker for R DCF trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DcfLanguage {}

impl Language for DcfLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        match raw.0 {
            0 => SyntaxKind::Root,
            1 => SyntaxKind::Record,
            2 => SyntaxKind::Field,
            3 => SyntaxKind::Error,
            4 => SyntaxKind::Name,
            5 => SyntaxKind::Colon,
            6 => SyntaxKind::Value,
            7 => SyntaxKind::Indent,
            8 => SyntaxKind::Dot,
            9 => SyntaxKind::Whitespace,
            10 => SyntaxKind::LineEnding,
            11 => SyntaxKind::ErrorToken,
            _ => panic!("invalid R DCF syntax kind"),
        }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

/// A typed Rowan syntax node.
pub type SyntaxNode = rowan::SyntaxNode<DcfLanguage>;
/// A typed Rowan syntax token.
pub type SyntaxToken = rowan::SyntaxToken<DcfLanguage>;
/// A typed Rowan node-or-token element.
pub type SyntaxElement = rowan::NodeOrToken<SyntaxNode, SyntaxToken>;
