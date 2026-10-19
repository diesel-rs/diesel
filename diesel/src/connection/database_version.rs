use core::fmt;
use core::str::FromStr;

/// The version of the database a connection talks to, ordered by major, then minor, then patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DatabaseVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl DatabaseVersion {
    /// Creates the version `major.minor.patch`.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// The major version.
    pub const fn major(&self) -> u32 {
        self.major
    }

    /// The minor version.
    pub const fn minor(&self) -> u32 {
        self.minor
    }

    /// The patch version.
    pub const fn patch(&self) -> u32 {
        self.patch
    }
}

impl fmt::Display for DatabaseVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Parses `major.minor.patch`, the format [`Display`](fmt::Display) writes, with every component
/// made of decimal digits only.
///
/// ```rust
/// use diesel::connection::DatabaseVersion;
///
/// assert_eq!("8.4.11".parse(), Ok(DatabaseVersion::new(8, 4, 11)));
/// assert!("8.4".parse::<DatabaseVersion>().is_err());
/// ```
impl FromStr for DatabaseVersion {
    type Err = ParseDatabaseVersionError;

    fn from_str(version: &str) -> Result<Self, Self::Err> {
        let mut components = version.split('.').map(|component| {
            if component.bytes().all(|byte| byte.is_ascii_digit()) {
                component.parse().ok()
            } else {
                None
            }
        });
        match (
            components.next(),
            components.next(),
            components.next(),
            components.next(),
        ) {
            (Some(Some(major)), Some(Some(minor)), Some(Some(patch)), None) => {
                Ok(Self::new(major, minor, patch))
            }
            _ => Err(ParseDatabaseVersionError),
        }
    }
}

/// The error returned when a string is not a [`DatabaseVersion`] in `major.minor.patch` form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseDatabaseVersionError;

impl fmt::Display for ParseDatabaseVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid database version, expected `major.minor.patch`")
    }
}

impl core::error::Error for ParseDatabaseVersionError {}

#[cfg(test)]
mod tests {
    use super::{DatabaseVersion, ParseDatabaseVersionError};

    #[diesel_test_helper::test]
    fn versions_order_by_major_then_minor_then_patch() {
        assert!(DatabaseVersion::new(10, 0, 0) > DatabaseVersion::new(9, 6, 20));
        assert!(DatabaseVersion::new(3, 10, 0) > DatabaseVersion::new(3, 9, 9));
        assert!(DatabaseVersion::new(3, 9, 10) > DatabaseVersion::new(3, 9, 9));
    }

    #[diesel_test_helper::test]
    fn parsing_inverts_display() {
        for version in [
            DatabaseVersion::new(0, 0, 0),
            DatabaseVersion::new(8, 4, 11),
            DatabaseVersion::new(u32::MAX, u32::MAX, u32::MAX),
        ] {
            assert_eq!(version.to_string().parse(), Ok(version));
        }
    }

    #[diesel_test_helper::test]
    fn parsing_rejects_anything_but_three_decimal_components() {
        for input in [
            "",
            "8",
            "8.4",
            "8.4.11.2",
            "8..4",
            "8..4.11",
            ".8.4.11",
            "8.4.11.",
            "+8.4.11",
            "8.+4.11",
            "8.4.-1",
            " 8.4.11",
            "8.4.11 ",
            "8.4.4294967296",
            "8.4.11-MariaDB",
        ] {
            assert_eq!(
                input.parse::<DatabaseVersion>(),
                Err(ParseDatabaseVersionError),
                "{input:?}"
            );
        }
    }
}
