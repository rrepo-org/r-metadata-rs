//! R package versions.

use crate::Span;
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    str::FromStr,
};

/// A numeric R package version that retains its original spelling.
///
/// At least two ASCII decimal components are required. Dots and hyphens are
/// equivalent separators. Equality, ordering and hashing ignore spelling,
/// leading zeroes and trailing zero components.
#[derive(Debug, Clone)]
pub struct Version {
    original: Box<str>,
    components: Box<[u32]>,
}

impl Version {
    /// Returns the exact parsed spelling.
    pub fn as_str(&self) -> &str {
        &self.original
    }
    /// Returns numeric components.
    pub fn components(&self) -> &[u32] {
        &self.components
    }
    /// Returns the first component.
    pub fn major(&self) -> u32 {
        self.components[0]
    }
    /// Returns the second component.
    pub fn minor(&self) -> u32 {
        self.components[1]
    }

    fn significant(&self) -> &[u32] {
        let len = self
            .components
            .iter()
            .rposition(|&x| x != 0)
            .map_or(0, |i| i + 1);
        &self.components[..len]
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl Eq for Version {}
impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.significant().hash(state);
    }
}
impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (0..self.components.len().max(other.components.len()))
            .map(|i| {
                self.components
                    .get(i)
                    .copied()
                    .unwrap_or(0)
                    .cmp(&other.components.get(i).copied().unwrap_or(0))
            })
            .find(|order| !order.is_eq())
            .unwrap_or(Ordering::Equal)
    }
}
impl AsRef<str> for Version {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Version {
    type Err = VersionParseError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut values = Vec::new();
        let mut start = 0;
        for (offset, byte) in input.bytes().enumerate() {
            match byte {
                b'0'..=b'9' => {}
                b'.' | b'-' => {
                    if offset == start {
                        return Err(VersionParseError::EmptyComponent {
                            index: values.len(),
                            span: Span::new(offset, offset),
                        });
                    }
                    values.push(component(input, values.len(), start, offset)?);
                    start = offset + 1;
                }
                _ => {
                    let end = offset + input[offset..].chars().next().map_or(1, char::len_utf8);
                    return Err(VersionParseError::InvalidComponent {
                        index: values.len(),
                        span: Span::new(offset, end),
                    });
                }
            }
        }
        if start == input.len() {
            return Err(VersionParseError::EmptyComponent {
                index: values.len(),
                span: Span::new(start, start),
            });
        }
        values.push(component(input, values.len(), start, input.len())?);
        if values.len() < 2 {
            return Err(VersionParseError::TooFewComponents);
        }
        Ok(Self {
            original: input.into(),
            components: values.into_boxed_slice(),
        })
    }
}

fn component(
    input: &str,
    index: usize,
    start: usize,
    end: usize,
) -> Result<u32, VersionParseError> {
    input[start..end]
        .parse()
        .map_err(|_| VersionParseError::ComponentOverflow {
            index,
            span: Span::new(start, end),
        })
}

/// Error parsing a numeric R package version.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VersionParseError {
    /// Fewer than two components were supplied.
    #[error("version must contain at least two components")]
    TooFewComponents,
    /// A component is empty.
    #[error("version component {index} is empty")]
    EmptyComponent {
        /// Zero-based component index.
        index: usize,
        /// Offending relative byte span.
        span: Span,
    },
    /// A component contains a non-ASCII digit.
    #[error("version component {index} is not an ASCII integer")]
    InvalidComponent {
        /// Zero-based component index.
        index: usize,
        /// Offending relative byte span.
        span: Span,
    },
    /// A component exceeds `u32`.
    #[error("version component {index} is too large")]
    ComponentOverflow {
        /// Zero-based component index.
        index: usize,
        /// Offending relative byte span.
        span: Span,
    },
}

impl VersionParseError {
    /// Returns the offending relative byte span, when localized.
    pub const fn span(&self) -> Option<Span> {
        match self {
            Self::TooFewComponents => None,
            Self::EmptyComponent { span, .. }
            | Self::InvalidComponent { span, .. }
            | Self::ComponentOverflow { span, .. } => Some(*span),
        }
    }
    /// Alias for [`Self::span`].
    pub const fn range(&self) -> Option<Span> {
        self.span()
    }
}
