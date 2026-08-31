//! Lossless, failure-tolerant access to R package `DESCRIPTION` files.
//!
//! [`Description`] preserves the input exactly. Raw accessors never parse
//! semantic values; their typed counterparts report errors for only the field
//! declaration being inspected.
#![forbid(unsafe_code)]

mod builder;
mod typed;
mod validation;

use std::{fmt, str::Utf8Error};

pub use builder::DescriptionBuilder;
pub use r_dcf_syntax::{
    Diagnostic as SyntaxDiagnostic, DiagnosticKind as SyntaxDiagnosticKind, EditError, Field,
    FieldName, FormatStyle, InvalidFieldName, InvalidLogicalValue, LineEnding, LogicalValue, Parse,
    Record, SourceSpan, ValueText,
};
pub use typed::{CollectionEntry, CollectionIssue, CollectionResult};
pub use validation::{Severity, Validation, ValidationIssue};

/// A lossless parsed R package `DESCRIPTION` document.
///
/// The type owns only the persistent syntax parse. Cloning is inexpensive and
/// the type is safe to move or share between threads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Description {
    parse: Parse,
}

impl Description {
    /// Starts a canonical `DESCRIPTION` builder.
    pub fn builder() -> DescriptionBuilder {
        DescriptionBuilder::default()
    }

    /// Parses UTF-8 text infallibly, retaining malformed text and diagnostics.
    pub fn parse(source: &str) -> Self {
        Self {
            parse: r_dcf_syntax::parse(source),
        }
    }

    /// Parses bytes after validating UTF-8.
    ///
    /// # Errors
    ///
    /// Returns the standard UTF-8 error if `source` is not valid UTF-8.
    pub fn parse_utf8(source: &[u8]) -> Result<Self, Utf8Error> {
        std::str::from_utf8(source).map(Self::parse)
    }

    /// Wraps an existing DCF parse.
    pub fn from_parse(parse: Parse) -> Self {
        Self { parse }
    }

    /// Returns the owned lower-level parse.
    pub const fn as_parse(&self) -> &Parse {
        &self.parse
    }

    /// Consumes this wrapper and returns the lower-level parse.
    pub fn into_parse(self) -> Parse {
        self.parse
    }

    /// Returns syntax diagnostics in source order.
    pub fn diagnostics(&self) -> &[SyntaxDiagnostic] {
        self.parse.diagnostics()
    }

    /// Iterates over all records in source order.
    pub fn records(&self) -> impl Iterator<Item = Record> {
        self.parse.records()
    }

    /// Returns the first record, conventionally the package record.
    pub fn primary_record(&self) -> Option<Record> {
        self.records().next()
    }

    /// Returns the last case-sensitive match in the primary record.
    pub fn field(&self, name: &str) -> Option<Field> {
        self.primary_record()?.last_field(name)
    }

    /// Iterates over case-sensitive matches in the primary record.
    pub fn fields<'a>(&'a self, name: &'a str) -> impl Iterator<Item = Field> + 'a {
        self.primary_record()
            .into_iter()
            .flat_map(move |record| record.fields().collect::<Vec<_>>())
            .filter(move |field| field.name().as_deref() == Some(name))
    }

    /// Iterates over every valid field in every record in source order.
    pub fn fields_all(&self) -> impl Iterator<Item = Field> {
        self.records()
            .flat_map(|record| record.fields().collect::<Vec<_>>())
    }

    fn raw(&self, name: &str) -> Option<ValueText> {
        self.field(name).map(|field| field.value())
    }

    /// Replaces the last matching field in the primary record.
    ///
    /// # Errors
    ///
    /// Returns an error if the primary record or matching field is absent.
    pub fn replace_last(&self, name: &str, value: &LogicalValue) -> Result<Self, EditError> {
        self.parse
            .replace_last(0, name, value)
            .map(Self::from_parse)
    }

    /// Replaces all matching fields in the primary record.
    ///
    /// # Errors
    ///
    /// Returns an error if the primary record or matching field is absent.
    pub fn replace_all(&self, name: &str, value: &LogicalValue) -> Result<Self, EditError> {
        self.parse.replace_all(0, name, value).map(Self::from_parse)
    }

    /// Inserts a field after the last case-sensitive anchor in the primary record.
    ///
    /// # Errors
    ///
    /// Returns an error if the primary record or anchor field is absent.
    pub fn insert_after(
        &self,
        after: &str,
        name: &FieldName,
        value: &LogicalValue,
    ) -> Result<Self, EditError> {
        self.parse
            .insert_after(0, after, name, value)
            .map(Self::from_parse)
    }

    /// Removes the last matching field from the primary record.
    ///
    /// # Errors
    ///
    /// Returns an error if the primary record or matching field is absent.
    pub fn remove_last(&self, name: &str) -> Result<Self, EditError> {
        self.parse.remove_last(0, name).map(Self::from_parse)
    }

    /// Removes all matching fields from the primary record.
    ///
    /// # Errors
    ///
    /// Returns an error if the primary record or matching field is absent.
    pub fn remove_all(&self, name: &str) -> Result<Self, EditError> {
        self.parse.remove_all(0, name).map(Self::from_parse)
    }

    /// Replaces the last matching field, or inserts it after the final field.
    ///
    /// # Errors
    ///
    /// Returns an error when there is no primary record. Insertion into an
    /// existing but fieldless malformed record is intentionally unsupported.
    pub fn set_field(&self, name: &FieldName, value: &LogicalValue) -> Result<Self, EditError> {
        if self.field(name.as_str()).is_some() {
            return self.replace_last(name.as_str(), value);
        }
        let record = self
            .primary_record()
            .ok_or(EditError::RecordOutOfBounds { index: 0 })?;
        let anchor = record
            .fields()
            .last()
            .and_then(|field| field.name())
            .ok_or_else(|| EditError::FieldNotFound {
                name: "<insertion anchor>".to_owned(),
            })?;
        self.insert_after(&anchor, name, value)
    }

    /// Alias for [`Self::set_field`].
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::set_field`].
    pub fn upsert(&self, name: &FieldName, value: &LogicalValue) -> Result<Self, EditError> {
        self.set_field(name, value)
    }
}

impl fmt::Display for Description {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.parse, formatter)
    }
}

macro_rules! raw_accessors {
    ($($method:ident => $field:literal),+ $(,)?) => {$(
        #[doc = concat!("Returns raw `", $field, "` from the last declaration.")]
        pub fn $method(&self) -> Option<ValueText> { self.raw($field) }
    )+};
}

impl Description {
    raw_accessors! {
        package => "Package", type_ => "Type", package_type => "Type", title => "Title", version => "Version",
        date => "Date", date_publication => "Date/Publication", authors_at_r => "Authors@R",
        author => "Author", maintainer => "Maintainer", copyright => "Copyright",
        description => "Description",
        license => "License", depends => "Depends", imports => "Imports", suggests => "Suggests",
        enhances => "Enhances", linking_to => "LinkingTo", system_requirements => "SystemRequirements",
        url => "URL", bug_reports => "BugReports", additional_repositories => "Additional_repositories",
        remotes => "Remotes", encoding => "Encoding", repository => "Repository",
        contact => "Contact", mailing_list => "MailingList", note => "Note",
        needs_compilation => "NeedsCompilation",
        os_type => "OS_type", priority => "Priority", archs => "Archs", biarch => "Biarch",
        classification_acm => "Classification/ACM", classification_acm_2012 => "Classification/ACM-2012",
        classification_jel => "Classification/JEL", classification_msc => "Classification/MSC",
        classification_msc_2010 => "Classification/MSC-2010", collate => "Collate",
        lazy_data => "LazyData", lazy_load => "LazyLoad", byte_compile => "ByteCompile",
        keep_source => "KeepSource", use_lto => "UseLTO", staged_install => "StagedInstall",
        zip_data => "ZipData", build_vignettes => "BuildVignettes",
        license_is_foss => "License_is_FOSS", license_restricts_use => "License_restricts_use",
        vignette_builder => "VignetteBuilder", roxygen_note => "RoxygenNote", rd_macros => "RdMacros",
        packaged => "Packaged", built => "Built",
    }
}

#[cfg(test)]
mod tests;
