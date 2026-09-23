//! Lossless single-record conversions to and from DESCRIPTION documents.

use r_description::Description;

use crate::PackageRecord;

/// A DESCRIPTION document cannot be represented as a single package record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RecordConversionError {
    /// The document contains no records (it is empty or contains only blank lines).
    #[error("DESCRIPTION contains no records")]
    NoRecords,
    /// The document contains multiple records; conversion would discard metadata.
    #[error("DESCRIPTION contains multiple records")]
    MultipleRecords,
}

/// Converts the exact record text into a standalone DESCRIPTION document.
///
/// All fields (including custom fields and duplicates), their order, and malformed
/// text are retained. Lookup remains case-sensitive and last-declaration-wins.
/// No semantic validation is performed and no missing metadata is invented.
///
/// Record-local spacing, continuation indentation, line endings, and the presence
/// or absence of a final newline are preserved. Surrounding blank lines and
/// record separators are document-level text and are not included. Syntax
/// diagnostics are recomputed relative to the standalone document.
impl From<&PackageRecord> for Description {
    fn from(record: &PackageRecord) -> Self {
        Self::parse(&record.to_string())
    }
}

/// Owned equivalent of the lossless borrowed conversion.
impl From<PackageRecord> for Description {
    fn from(record: PackageRecord) -> Self {
        Self::from(&record)
    }
}

/// Converts a DESCRIPTION containing exactly one record into a package record.
///
/// The existing syntax record is retained, including unknown fields, duplicate
/// declarations, malformed text, and exact record-local formatting. No semantic
/// validation is performed: missing identity fields and invalid field values are
/// accepted. Lookup remains case-sensitive and last-declaration-wins.
/// Surrounding document-level blank lines are not part of the returned record.
///
/// Empty/blank documents return [`RecordConversionError::NoRecords`]; documents
/// with multiple records return [`RecordConversionError::MultipleRecords`].
impl TryFrom<&Description> for PackageRecord {
    type Error = RecordConversionError;

    fn try_from(description: &Description) -> Result<Self, Self::Error> {
        let mut records = description.records();
        let record = records.next().ok_or(RecordConversionError::NoRecords)?;
        if records.next().is_some() {
            return Err(RecordConversionError::MultipleRecords);
        }
        Ok(Self { record })
    }
}

/// Owned equivalent of the lossless borrowed conversion.
impl TryFrom<Description> for PackageRecord {
    type Error = RecordConversionError;

    fn try_from(description: Description) -> Result<Self, Self::Error> {
        Self::try_from(&description)
    }
}
