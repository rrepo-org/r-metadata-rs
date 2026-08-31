use std::{error::Error, fmt};

use r_dcf_syntax::{FieldName, InvalidFieldName, InvalidLogicalValue, LogicalValue, make};
use r_metadata::{Version, VersionParseError};

use crate::{FormatStyle, Packages, validation::valid_package_name};

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

/// A structurally validated package record under construction.
#[derive(Debug, Clone)]
pub struct RecordBuilder {
    fields: Vec<(FieldName, LogicalValue)>,
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
        let mut builder = Self { fields: Vec::new() };
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

    pub(crate) fn render(&self, style: &FormatStyle) -> String {
        let style = clean_style(style);
        let fields = self
            .fields
            .iter()
            .map(|(name, value)| make::field(name, value, &style))
            .collect::<Vec<_>>();
        make::record(&fields, &style)
    }
}

/// A builder for a structurally clean zero-or-more-record `PACKAGES` file.
#[derive(Debug, Clone, Default)]
pub struct PackagesBuilder {
    records: Vec<RecordBuilder>,
    style: FormatStyle,
}

impl PackagesBuilder {
    /// Creates an empty builder using the default format style.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets formatting used for every subsequently rendered field and record.
    pub fn format_style(mut self, style: FormatStyle) -> Self {
        self.style = style;
        self
    }

    /// Appends one validated record.
    pub fn record(mut self, record: RecordBuilder) -> Self {
        self.records.push(record);
        self
    }

    /// Constructs the persistent document directly from clean rendered text.
    pub fn build(self) -> Packages {
        let style = clean_style(&self.style);
        let records = self
            .records
            .iter()
            .map(|record| record.render(&style))
            .collect::<Vec<_>>();
        Packages::parse(&make::document(&records, &style))
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
