//! Semantic values used in R package metadata.
//!
//! Collection parsers in this crate recover at delimiters: malformed entries
//! are reported as positioned issues without discarding valid neighbours.

#![forbid(unsafe_code)]

mod relation;
mod remote;
mod scalar;
mod urls;
mod version;

pub use relation::{
    InvalidRelationPackage, PositionedRelationParseError, Relation, RelationList,
    RelationParseError, RequirementVersion, RequirementVersionParseError, Revision,
    RevisionParseError, VersionRequirement,
};
pub use remote::{
    BioconductorCredentials, BioconductorRemote, CranRemote, GenericGitRemote, HostedGitRemote,
    LocalRemote, PositionedRemoteParseError, Remote, RemoteList, RemoteParseError, RemoteSource,
    SvnRemote, UnknownRemote, UrlRemote,
};
pub use scalar::{
    Logical, LogicalParseError, OsType, OsTypeParseError, Priority, PriorityParseError,
};
pub use url::Url;
pub use urls::{
    AdditionalRepositories, PositionedUrlParseError, SpannedUrl, UrlList, UrlParseError,
};
pub use version::{Version, VersionParseError};

/// A half-open UTF-8 byte span into parser input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    /// Creates a span. `start` and `end` are byte offsets.
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns the inclusive start byte offset.
    pub const fn start(self) -> usize {
        self.start
    }

    /// Returns the exclusive end byte offset.
    pub const fn end(self) -> usize {
        self.end
    }

    /// Returns whether this span has zero length.
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Returns the span length in bytes.
    pub const fn len(self) -> usize {
        self.end - self.start
    }
}

/// A semantic value and its half-open byte span in the parsed collection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spanned<T> {
    /// Parsed value.
    pub value: T,
    /// Span covering the complete trimmed entry.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Returns the parsed value.
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Returns the value's source span.
    pub const fn span(&self) -> Span {
        self.span
    }
}
