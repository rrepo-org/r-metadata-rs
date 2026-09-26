use std::{borrow::Borrow, error::Error, fmt};

use r_dcf_syntax::{FieldName, InvalidFieldName, InvalidLogicalValue, LogicalValue, make};
use r_metadata::{Version, VersionParseError};

use crate::{FormatStyle, LineEnding, PackageRecord, Packages, validation::valid_package_name};

/// An invalid value supplied to a structural builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// The field name is not valid DCF syntax.
    FieldName(InvalidFieldName),
    /// The logical value cannot be represented losslessly by the builder.
    Value(InvalidLogicalValue),
    /// The required package name is invalid.
    PackageName,
    /// The required package version is invalid.
    Version(VersionParseError),
}

impl fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldName(error) => error.fmt(formatter),
            Self::Value(error) => error.fmt(formatter),
            Self::PackageName => formatter.write_str("invalid package name"),
            Self::Version(error) => error.fmt(formatter),
        }
    }
}

impl Error for BuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::FieldName(error) => Some(error),
            Self::Value(error) => Some(error),
            Self::Version(error) => Some(error),
            Self::PackageName => None,
        }
    }
}

/// Builds structurally valid DCF with checked initial package and version fields.
/// Full semantic validation remains explicit, including for duplicate fields.
#[derive(Debug, Clone)]
pub struct RecordBuilder {
    fields: Vec<(FieldName, LogicalValue)>,
    style: FormatStyle,
}

impl RecordBuilder {
    /// Starts a record with validated required `Package` and `Version` fields.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid package name or version.
    pub fn new(package: &str, version: &str) -> Result<Self, BuildError> {
        if !valid_package_name(package) {
            return Err(BuildError::PackageName);
        }
        let version: Version = version.parse().map_err(BuildError::Version)?;
        let mut builder = Self {
            fields: Vec::new(),
            style: FormatStyle::default(),
        };
        builder.push_valid("Package", package)?;
        builder.push_valid("Version", version.as_str())?;
        Ok(builder)
    }

    /// Appends a validated field, preserving duplicate fields and order.
    ///
    /// # Errors
    ///
    /// Returns an error when the name or logical value is not representable.
    pub fn field(mut self, name: &str, value: &str) -> Result<Self, BuildError> {
        self.push_valid(name, value)?;
        Ok(self)
    }

    fn push_valid(&mut self, name: &str, value: &str) -> Result<(), BuildError> {
        let name = FieldName::new(name).map_err(BuildError::FieldName)?;
        let value = LogicalValue::new(value).map_err(BuildError::Value)?;
        self.fields.push((name, value));
        Ok(())
    }

    /// Sets formatting for all fields in this record.
    /// Empty or non-whitespace continuation indentation is replaced with a space.
    pub fn format_style(mut self, style: FormatStyle) -> Self {
        self.style = style;
        self
    }

    /// Builds a standalone record without a trailing newline.
    /// Does not perform full semantic validation or normalize field values.
    ///
    /// # Panics
    ///
    /// Panics if the rendered record exceeds the syntax tree's 4 GiB size limit.
    pub fn build(self) -> PackageRecord {
        let style = clean_style(&self.style);
        let fields = self
            .fields
            .iter()
            .map(|(name, value)| make::field(name, value, &style))
            .collect::<Vec<_>>();
        let parse = r_dcf_syntax::parse(&make::record(&fields, &style));
        PackageRecord {
            record: parse
                .records()
                .next()
                .expect("required fields form one record"),
        }
    }
}

/// A builder for a zero-or-more-record `PACKAGES` file.
///
/// All records are preserved without normalization or validation, regardless of
/// whether they were parsed, converted from DESCRIPTION, or built independently.
#[derive(Debug, Clone, Default)]
pub struct PackagesBuilder {
    records: Vec<String>,
    line_ending: LineEnding,
}

impl PackagesBuilder {
    /// Creates an empty builder using LF for unterminated record boundaries.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets boundary line endings when the preceding record has no trailing newline.
    /// Applies to all records in this builder; record-local text is unaffected.
    pub fn line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Appends an owned or borrowed record, preserving its exact text.
    /// No normalization, filtering, or validation is performed.
    pub fn record(mut self, record: impl Borrow<PackageRecord>) -> Self {
        self.records.push(record.borrow().to_string());
        self
    }

    /// Appends owned or borrowed records in iteration order.
    pub fn records<R: Borrow<PackageRecord>>(
        mut self,
        records: impl IntoIterator<Item = R>,
    ) -> Self {
        self.records.extend(
            records
                .into_iter()
                .map(|record| record.borrow().to_string()),
        );
        self
    }

    /// Constructs the document in linear time and memory in the total text size.
    ///
    /// Empty input produces empty text. Existing record text is retained verbatim,
    /// with one blank line inserted between records. A preceding record's trailing
    /// line ending supplies the separator convention; if absent, two configured
    /// line endings are inserted. The final record's newline state is preserved.
    /// The completed text is parsed once, retaining malformed content and findings.
    pub fn build(self) -> Packages {
        let mut output = String::new();
        for record in self.records {
            separate_record(&mut output, self.line_ending);
            output.push_str(&record);
        }
        Packages::parse(&output)
    }
}

/// Completes a blank separator line, preserving all existing document text.
/// Only inspects the final physical line, never the accumulated document prefix.
pub(crate) fn separate_record(output: &mut String, fallback: LineEnding) {
    if output.is_empty() {
        return;
    }
    let ending = trailing_line_ending(output);
    let body_end = output.len() - ending.map_or(0, str::len);
    let prefix = &output[..body_end];
    let body_start = prefix.rfind(['\r', '\n']).map_or(0, |index| index + 1);
    let blank = prefix[body_start..]
        .bytes()
        .all(|byte| matches!(byte, b' ' | b'\t' | 0x0c));
    if blank {
        if ending.is_none() {
            let separator =
                trailing_line_ending(&prefix[..body_start]).unwrap_or(fallback.as_str());
            output.push_str(separator);
        }
    } else if let Some(ending) = ending {
        // Reusing a lone CR avoids coalescing it with LF and losing the blank line.
        output.push_str(ending);
    } else {
        output.push_str(fallback.as_str());
        output.push_str(fallback.as_str());
    }
}

fn trailing_line_ending(text: &str) -> Option<&'static str> {
    if text.ends_with("\r\n") {
        Some("\r\n")
    } else if text.ends_with('\r') {
        Some("\r")
    } else if text.ends_with('\n') {
        Some("\n")
    } else {
        None
    }
}

fn clean_style(style: &FormatStyle) -> FormatStyle {
    let mut style = style.clone();
    if style.continuation_indent.is_empty()
        || !style
            .continuation_indent
            .bytes()
            .all(|byte| matches!(byte, b' ' | b'\t'))
    {
        " ".clone_into(&mut style.continuation_indent);
    }
    style
}
