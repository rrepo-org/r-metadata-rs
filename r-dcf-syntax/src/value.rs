//! Validated semantic value types.

use std::{error::Error, fmt, sync::Arc};

/// A validated, case-sensitive DCF field name.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldName(Arc<str>);

impl FieldName {
    /// Validates and stores `name`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldName`] if the name is not valid DCF syntax.
    pub fn new(name: impl AsRef<str>) -> Result<Self, InvalidFieldName> {
        let name = name.as_ref();
        if is_valid_field_name(name) {
            Ok(Self(Arc::from(name)))
        } else {
            Err(InvalidFieldName)
        }
    }

    /// Returns the field name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for FieldName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("FieldName")
            .field(&self.as_str())
            .finish()
    }
}

impl fmt::Display for FieldName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for FieldName {
    type Error = InvalidFieldName;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for FieldName {
    type Error = InvalidFieldName;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Error returned for a field name outside the portable R DCF grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidFieldName;

impl fmt::Display for InvalidFieldName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid DCF field name")
    }
}

impl Error for InvalidFieldName {}

/// An owned logical value, with physical line endings excluded.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct LogicalValue(Arc<str>);

impl LogicalValue {
    /// Creates a logical value. LF separates logical lines; CR is rejected.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidLogicalValue`] if the text cannot be represented as a
    /// logical DCF value.
    pub fn new(value: impl AsRef<str>) -> Result<Self, InvalidLogicalValue> {
        let value = value.as_ref();
        let lines_are_canonical = value.split('\n').enumerate().all(|(index, line)| {
            line == line.trim_matches([' ', '\t']) && (index == 0 || line != ".")
        });
        if value.contains('\r') || value.contains('\0') || !lines_are_canonical {
            Err(InvalidLogicalValue)
        } else {
            Ok(Self(Arc::from(value)))
        }
    }

    /// Returns the unfolded logical text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LogicalValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for LogicalValue {
    type Error = InvalidLogicalValue;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for LogicalValue {
    type Error = InvalidLogicalValue;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Error returned when logical text cannot be represented without ambiguity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidLogicalValue;

impl fmt::Display for InvalidLogicalValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "logical DCF values cannot contain CR, NUL, surrounding horizontal whitespace, or a continued dot-only line",
        )
    }
}

impl Error for InvalidLogicalValue {}

pub(crate) fn is_valid_field_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric())
        && chars.all(|character| character.is_ascii_graphic() && character != ':')
}
