//! Lossless, failure-tolerant access to R repository `PACKAGES` files.
//!
//! Parsing never rejects DCF syntax. Use [`Packages::validate`] when semantic
//! findings are wanted, and the typed methods on [`PackageRecord`] when only a
//! particular field should be interpreted.

#![forbid(unsafe_code)]

mod builder;
mod edit;
mod validation;

use std::{fmt, str::Utf8Error};

use r_dcf_syntax::{Field, Parse, ValueText};
use r_metadata::{
    Logical, LogicalParseError, OsType, OsTypeParseError, Priority, PriorityParseError,
    RelationList, UrlList, Version, VersionParseError,
};

pub use builder::{BuildError, PackagesBuilder, RecordBuilder};
pub use edit::EditError;
pub use r_dcf_syntax::{FormatStyle, LineEnding};
pub use validation::{Finding, FindingKind};

/// A persistent, lossless parse of zero or more repository package records.
///
/// This type owns only the syntax crate's compact [`Parse`]. Materialized
/// records and fields are transient wrappers around that persistent tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packages {
    parse: Parse,
}

impl Packages {
    /// Starts a canonical `PACKAGES` builder.
    pub fn builder() -> PackagesBuilder {
        PackagesBuilder::new()
    }

    /// Parses UTF-8 text without failing on malformed DCF syntax.
    pub fn parse(source: &str) -> Self {
        Self {
            parse: r_dcf_syntax::parse(source),
        }
    }

    /// Parses bytes after validating UTF-8.
    ///
    /// # Errors
    ///
    /// Returns the UTF-8 error without attempting a lossy conversion.
    pub fn parse_utf8(source: &[u8]) -> Result<Self, Utf8Error> {
        std::str::from_utf8(source).map(Self::parse)
    }

    /// Returns the number of records, including semantically invalid records.
    pub fn len(&self) -> usize {
        self.parse.records().count()
    }

    /// Returns whether the file has no records.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterates over transient records in source order.
    pub fn records(&self) -> impl Iterator<Item = PackageRecord> {
        self.parse.records().map(|record| PackageRecord { record })
    }

    /// Returns the record at a zero-based index.
    pub fn record(&self, index: usize) -> Option<PackageRecord> {
        self.records().nth(index)
    }
}

impl fmt::Display for Packages {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.parse, formatter)
    }
}

/// Parses UTF-8 text into a lossless [`Packages`] value.
pub fn parse(source: &str) -> Packages {
    Packages::parse(source)
}

/// Parses bytes into [`Packages`] after validating UTF-8.
///
/// # Errors
///
/// Returns an error when `source` is not UTF-8.
pub fn parse_utf8(source: &[u8]) -> Result<Packages, Utf8Error> {
    Packages::parse_utf8(source)
}

/// A transient view of one package record.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageRecord {
    record: r_dcf_syntax::Record,
}

impl fmt::Display for PackageRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.record, formatter)
    }
}

impl PackageRecord {
    /// Returns the last value whose field name exactly equals `name`.
    pub fn field(&self, name: &str) -> Option<ValueText> {
        self.record.last_field(name).map(|field| field.value())
    }

    /// Iterates over all values whose field names exactly equal `name`.
    pub fn fields<'a>(&'a self, name: &'a str) -> impl Iterator<Item = ValueText> + 'a {
        self.record.fields_named(name).map(|field| field.value())
    }

    /// Iterates over all structurally valid DCF fields in source order.
    pub fn all_fields(&self) -> impl Iterator<Item = Field> + '_ {
        self.record.fields()
    }

    /// Parses the last `Version` value, without inspecting any other field.
    pub fn parsed_version(&self) -> Option<Result<Version, VersionParseError>> {
        self.field("Version").map(|value| value.as_str().parse())
    }

    /// Parses the last named field as a recoverable relation list.
    pub fn relations(&self, name: &str) -> Option<RelationList> {
        self.field(name)
            .map(|value| RelationList::parse(value.as_str()))
    }

    /// Parses the last named field as an R logical scalar.
    pub fn logical(&self, name: &str) -> Option<Result<Logical, LogicalParseError>> {
        self.field(name).map(|value| value.as_str().parse())
    }

    /// Parses the last named field as a recoverable URL list.
    pub fn urls(&self, name: &str) -> Option<UrlList> {
        self.field(name).map(|value| UrlList::parse(value.as_str()))
    }

    /// Parses the last `Priority` value.
    pub fn parsed_priority(&self) -> Option<Result<Priority, PriorityParseError>> {
        self.field("Priority").map(|value| value.as_str().parse())
    }

    /// Parses the last `OS_type` value.
    pub fn parsed_os_type(&self) -> Option<Result<OsType, OsTypeParseError>> {
        self.field("OS_type").map(|value| value.as_str().parse())
    }

    /// Parses the last `Depends` relation list.
    pub fn parsed_depends(&self) -> Option<RelationList> {
        self.relations("Depends")
    }

    /// Parses the last `Imports` relation list.
    pub fn parsed_imports(&self) -> Option<RelationList> {
        self.relations("Imports")
    }

    /// Parses the last `Suggests` relation list.
    pub fn parsed_suggests(&self) -> Option<RelationList> {
        self.relations("Suggests")
    }

    /// Parses the last `Enhances` relation list.
    pub fn parsed_enhances(&self) -> Option<RelationList> {
        self.relations("Enhances")
    }

    /// Parses the last `LinkingTo` relation list.
    pub fn parsed_linking_to(&self) -> Option<RelationList> {
        self.relations("LinkingTo")
    }

    /// Parses the last `NeedsCompilation` logical value.
    pub fn parsed_needs_compilation(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("NeedsCompilation")
    }

    /// Parses the last `License_is_FOSS` logical value.
    pub fn parsed_license_is_foss(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("License_is_FOSS")
    }

    /// Parses the last `License_restricts_use` logical value.
    pub fn parsed_license_restricts_use(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("License_restricts_use")
    }

    /// Parses the last `URL` value as a recoverable URL list.
    pub fn parsed_url(&self) -> Option<UrlList> {
        self.urls("URL")
    }
}

macro_rules! raw_accessors {
    ($(($method:ident, $name:literal)),+ $(,)?) => {
        impl PackageRecord {
            $(
                #[doc = concat!("Returns the last raw `", $name, "` value.")]
                pub fn $method(&self) -> Option<ValueText> {
                    self.field($name)
                }
            )+
        }
    };
}

raw_accessors!(
    (package, "Package"),
    (version, "Version"),
    (priority, "Priority"),
    (depends, "Depends"),
    (imports, "Imports"),
    (suggests, "Suggests"),
    (enhances, "Enhances"),
    (linking_to, "LinkingTo"),
    (license, "License"),
    (license_is_foss, "License_is_FOSS"),
    (license_restricts_use, "License_restricts_use"),
    (os_type, "OS_type"),
    (archs, "Archs"),
    (md5sum, "MD5sum"),
    (needs_compilation, "NeedsCompilation"),
    (built, "Built"),
    (repository, "Repository"),
    (path, "Path"),
    (file, "File"),
    (packaged, "Packaged"),
    (date_publication, "Date/Publication"),
    (title, "Title"),
    (description, "Description"),
    (author, "Author"),
    (authors_at_r, "Authors@R"),
    (maintainer, "Maintainer"),
    (url, "URL"),
    (bug_reports, "BugReports"),
    (encoding, "Encoding"),
    (roxygen_note, "RoxygenNote"),
    (system_requirements, "SystemRequirements"),
    (additional_repositories, "Additional_repositories")
);

#[cfg(test)]
mod tests;
