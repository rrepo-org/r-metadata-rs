//! Small finite metadata values.

use std::str::FromStr;

/// An R logical value accepted by DESCRIPTION fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Logical(bool);
impl Logical {
    /// Returns the represented boolean.
    pub const fn get(self) -> bool {
        self.0
    }
}
impl From<Logical> for bool {
    fn from(value: Logical) -> Self {
        value.0
    }
}
impl From<bool> for Logical {
    fn from(value: bool) -> Self {
        Self(value)
    }
}
impl std::fmt::Display for Logical {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0 { "yes" } else { "no" })
    }
}
impl FromStr for Logical {
    type Err = LogicalParseError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.eq_ignore_ascii_case("yes") || value.eq_ignore_ascii_case("true") {
            Ok(Self(true))
        } else if value.eq_ignore_ascii_case("no") || value.eq_ignore_ascii_case("false") {
            Ok(Self(false))
        } else {
            Err(LogicalParseError)
        }
    }
}
/// Error parsing a logical value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("expected yes, true, no, or false")]
pub struct LogicalParseError;

/// Operating-system restriction from `OS_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OsType {
    /// Unix-like systems.
    Unix,
    /// Microsoft Windows.
    Windows,
}
impl std::fmt::Display for OsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unix => "unix",
            Self::Windows => "windows",
        })
    }
}
impl FromStr for OsType {
    type Err = OsTypeParseError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "unix" => Ok(Self::Unix),
            "windows" => Ok(Self::Windows),
            _ => Err(OsTypeParseError),
        }
    }
}
/// Error parsing `OS_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("expected unix or windows")]
pub struct OsTypeParseError;

/// Standard R package priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Priority {
    /// Base R package.
    Base,
    /// Recommended package.
    Recommended,
}
impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Base => "base",
            Self::Recommended => "recommended",
        })
    }
}
impl FromStr for Priority {
    type Err = PriorityParseError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "base" => Ok(Self::Base),
            "recommended" => Ok(Self::Recommended),
            _ => Err(PriorityParseError),
        }
    }
}
/// Error parsing package priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("expected base or recommended")]
pub struct PriorityParseError;
