use std::fmt;

/// Error information as given by the database
pub trait DatabaseErrorInformation {
    /// Get original error message
    fn message(&self) -> &str;

    /// Get optional error detals
    fn details(&self) -> Option<&str>;

    /// Get additional error hint, optional
    fn hint(&self) -> Option<&str>;

    /// Get name of the table this error concerns
    fn table_name(&self) -> Option<&str>;

    /// Get name of the column this error concerns
    fn column_name(&self) -> Option<&str>;

    /// Get name of the constraint this error concerns
    fn constraint_name(&self) -> Option<&str>;
}

impl fmt::Debug for DatabaseErrorInformation + Send {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.message(), f)
    }
}

impl DatabaseErrorInformation for String {
    fn message(&self) -> &str {
        &self
    }

    fn details(&self) -> Option<&str> { None }
    fn hint(&self) -> Option<&str> { None }
    fn table_name(&self) -> Option<&str> { None }
    fn column_name(&self) -> Option<&str> { None }
    fn constraint_name(&self) -> Option<&str> { None }
}
