use core::fmt;

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

#[cfg(test)]
mod tests {
    use super::DatabaseVersion;

    #[diesel_test_helper::test]
    fn versions_order_by_major_then_minor_then_patch() {
        assert!(DatabaseVersion::new(10, 0, 0) > DatabaseVersion::new(9, 6, 20));
        assert!(DatabaseVersion::new(3, 10, 0) > DatabaseVersion::new(3, 9, 9));
        assert!(DatabaseVersion::new(3, 9, 10) > DatabaseVersion::new(3, 9, 9));
    }
}
