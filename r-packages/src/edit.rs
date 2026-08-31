use std::{error::Error, fmt};

use r_dcf_syntax::{FieldName, InvalidFieldName, InvalidLogicalValue, LogicalValue};

use crate::{FormatStyle, Packages, RecordBuilder};

/// A failure to apply an immutable structured edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    /// The selected record does not exist.
    RecordOutOfBounds {
        /// Requested zero-based record index.
        index: usize,
    },
    /// No matching case-sensitive field exists.
    FieldNotFound {
        /// Requested case-sensitive field name.
        name: String,
    },
    /// A new field name is invalid.
    FieldName(InvalidFieldName),
    /// A replacement value cannot be represented as logical DCF text.
    Value(InvalidLogicalValue),
}

impl fmt::Display for EditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecordOutOfBounds { index } => {
                write!(formatter, "record index {index} is out of bounds")
            }
            Self::FieldNotFound { name } => write!(formatter, "field {name:?} was not found"),
            Self::FieldName(error) => error.fmt(formatter),
            Self::Value(error) => error.fmt(formatter),
        }
    }
}

impl Error for EditError {}

impl From<r_dcf_syntax::EditError> for EditError {
    fn from(error: r_dcf_syntax::EditError) -> Self {
        match error {
            r_dcf_syntax::EditError::RecordOutOfBounds { index } => {
                Self::RecordOutOfBounds { index }
            }
            r_dcf_syntax::EditError::FieldNotFound { name } => Self::FieldNotFound { name },
        }
    }
}

impl Packages {
    /// Replaces the last matching field in one record.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid value or missing record or field.
    pub fn replace_last(&self, record: usize, name: &str, value: &str) -> Result<Self, EditError> {
        let value = LogicalValue::new(value).map_err(EditError::Value)?;
        self.parse
            .replace_last(record, name, &value)
            .map(Self::from_parse)
            .map_err(Into::into)
    }

    /// Replaces every matching field in one record.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid value or missing record or field.
    pub fn replace_all(&self, record: usize, name: &str, value: &str) -> Result<Self, EditError> {
        let value = LogicalValue::new(value).map_err(EditError::Value)?;
        self.parse
            .replace_all(record, name, &value)
            .map(Self::from_parse)
            .map_err(Into::into)
    }

    /// Inserts a field after the last matching anchor in one record.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid input or a missing record or anchor.
    pub fn insert_after(
        &self,
        record: usize,
        after: &str,
        name: &str,
        value: &str,
    ) -> Result<Self, EditError> {
        let name = FieldName::new(name).map_err(EditError::FieldName)?;
        let value = LogicalValue::new(value).map_err(EditError::Value)?;
        self.parse
            .insert_after(record, after, &name, &value)
            .map(Self::from_parse)
            .map_err(Into::into)
    }

    /// Removes the last matching field in one record.
    ///
    /// # Errors
    ///
    /// Returns an error when the record or field is missing.
    pub fn remove_last(&self, record: usize, name: &str) -> Result<Self, EditError> {
        self.parse
            .remove_last(record, name)
            .map(Self::from_parse)
            .map_err(Into::into)
    }

    /// Removes every matching field in one record.
    ///
    /// # Errors
    ///
    /// Returns an error when the record or field is missing.
    pub fn remove_all(&self, record: usize, name: &str) -> Result<Self, EditError> {
        self.parse
            .remove_all(record, name)
            .map(Self::from_parse)
            .map_err(Into::into)
    }

    /// Appends a clean record using `style`, preserving all existing text.
    pub fn append_record(&self, record: &RecordBuilder, style: &FormatStyle) -> Self {
        let source = self.to_string();
        let rendered = record.render(style);
        let ending = style.line_ending.as_str();
        let has_blank_separator = source
            .strip_suffix(ending)
            .is_some_and(|prefix| prefix.ends_with(ending));
        let separator = if source.is_empty() || has_blank_separator {
            String::new()
        } else if source.ends_with(ending) {
            ending.to_owned()
        } else {
            format!("{ending}{ending}")
        };
        Self::parse(&format!("{source}{separator}{rendered}"))
    }

    /// Removes a complete record and its adjacent record separator.
    ///
    /// # Errors
    ///
    /// Returns an error when `index` is out of bounds.
    pub fn remove_record(&self, index: usize) -> Result<Self, EditError> {
        let records = self.parse.records().collect::<Vec<_>>();
        let selected = records
            .get(index)
            .ok_or(EditError::RecordOutOfBounds { index })?;
        let source = self.to_string();
        let range = if records.len() == 1 {
            selected.source_range().start..selected.source_range().end
        } else if let Some(next) = records.get(index + 1) {
            selected.source_range().start..next.source_range().start
        } else {
            records[index - 1].source_range().end..selected.source_range().end
        };
        let mut output = source;
        output.replace_range(range, "");
        Ok(Self::parse(&output))
    }

    fn from_parse(parse: r_dcf_syntax::Parse) -> Self {
        Self { parse }
    }
}
