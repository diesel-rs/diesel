use core::error::Error as StdError;
use core::fmt;
use core::str::FromStr;

/// A [SQLSTATE](https://en.wikipedia.org/wiki/SQLSTATE) error code, including
/// custom codes outside the standard, usable as a `match` pattern.
///
/// ```rust
/// use diesel::result::{SqlState, SqlStateClass};
///
/// const CUSTOM: SqlState = SqlState::new(*b"P1001").unwrap();
///
/// let code: SqlState = "P1001".parse().unwrap();
/// assert!(matches!(code, CUSTOM));
/// assert_eq!(code.as_str(), "P1001");
/// assert_eq!(code.class().as_str(), "P1");
/// assert!(!code.class().is_standard());
/// assert_eq!(code.subclass(), "001");
///
/// let code: SqlState = "23505".parse().unwrap();
/// assert_eq!(code.class(), SqlStateClass::INTEGRITY_CONSTRAINT_VIOLATION);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SqlState([u8; 5]);

impl SqlState {
    /// Returns `None` unless every byte is an ASCII digit or uppercase letter.
    pub const fn new(code: [u8; 5]) -> Option<Self> {
        let mut i = 0;
        while i < code.len() {
            if !matches!(code[i], b'0'..=b'9' | b'A'..=b'Z') {
                return None;
            }
            i += 1;
        }
        Some(Self(code))
    }

    /// The five character code.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.0).expect("a SQLSTATE consists of ASCII bytes")
    }

    /// The [`SqlStateClass`], made of the first two characters.
    pub fn class(&self) -> SqlStateClass {
        SqlStateClass([self.0[0], self.0[1]])
    }

    /// The subclass, made of the last three characters, where `000` means none.
    pub fn subclass(&self) -> &str {
        &self.as_str()[2..]
    }
}

impl fmt::Debug for SqlState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SqlState").field(&self.as_str()).finish()
    }
}

impl fmt::Display for SqlState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SqlState {
    type Err = InvalidSqlState;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        <[u8; 5]>::try_from(s.as_bytes())
            .ok()
            .and_then(Self::new)
            .ok_or(InvalidSqlState)
    }
}

/// The error parsing a [`SqlState`] from a string that is not five characters
/// from `0`-`9` and `A`-`Z`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidSqlState;

impl fmt::Display for InvalidSqlState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a SQLSTATE consists of five characters from `0`-`9` and `A`-`Z`")
    }
}

impl StdError for InvalidSqlState {}

/// The class of a [`SqlState`], with a constant for each class
/// [SQL:2011](https://en.wikipedia.org/wiki/SQLSTATE) defines.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SqlStateClass([u8; 2]);

impl SqlStateClass {
    /// The two character code of the class.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.0).expect("a SQLSTATE consists of ASCII bytes")
    }

    /// Whether the class starts with `0`-`4` or `A`-`H`, which the SQL standard reserves.
    pub fn is_standard(&self) -> bool {
        matches!(self.0[0], b'0'..=b'4' | b'A'..=b'H')
    }
}

impl fmt::Debug for SqlStateClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SqlStateClass")
            .field(&self.as_str())
            .finish()
    }
}

impl fmt::Display for SqlStateClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

macro_rules! sql_state_classes {
    ($($name:ident = $code:literal, $condition:literal;)*) => {
        impl SqlStateClass {
            $(
                #[doc = concat!("`", $code, "`, ", $condition, ".")]
                pub const $name: Self = {
                    let code = $code.as_bytes();
                    Self([code[0], code[1]])
                };
            )*
        }
    };
}

sql_state_classes! {
    SUCCESSFUL_COMPLETION = "00", "successful completion";
    WARNING = "01", "warning";
    NO_DATA = "02", "no data";
    DYNAMIC_SQL_ERROR = "07", "dynamic SQL error";
    CONNECTION_EXCEPTION = "08", "connection exception";
    TRIGGERED_ACTION_EXCEPTION = "09", "triggered action exception";
    FEATURE_NOT_SUPPORTED = "0A", "feature not supported";
    INVALID_TARGET_TYPE_SPECIFICATION = "0D", "invalid target type specification";
    INVALID_SCHEMA_NAME_LIST_SPECIFICATION = "0E", "invalid schema name list specification";
    LOCATOR_EXCEPTION = "0F", "locator exception";
    RESIGNAL_WHEN_HANDLER_NOT_ACTIVE = "0K", "resignal when handler not active";
    INVALID_GRANTOR = "0L", "invalid grantor";
    INVALID_SQL_INVOKED_PROCEDURE_REFERENCE = "0M", "invalid SQL-invoked procedure reference";
    SQL_XML_MAPPING_ERROR = "0N", "SQL/XML mapping error";
    INVALID_ROLE_SPECIFICATION = "0P", "invalid role specification";
    INVALID_TRANSFORM_GROUP_NAME_SPECIFICATION = "0S", "invalid transform group name specification";
    TARGET_TABLE_DISAGREES_WITH_CURSOR_SPECIFICATION = "0T", "target table disagrees with cursor specification";
    ATTEMPT_TO_ASSIGN_TO_NON_UPDATABLE_COLUMN = "0U", "attempt to assign to non-updatable column";
    ATTEMPT_TO_ASSIGN_TO_ORDERING_COLUMN = "0V", "attempt to assign to ordering column";
    PROHIBITED_STATEMENT_ENCOUNTERED_DURING_TRIGGER_EXECUTION = "0W", "prohibited statement encountered during trigger execution";
    INVALID_FOREIGN_SERVER_SPECIFICATION = "0X", "invalid foreign server specification";
    PASS_THROUGH_SPECIFIC_CONDITION = "0Y", "pass-through specific condition";
    DIAGNOSTICS_EXCEPTION = "0Z", "diagnostics exception";
    XQUERY_ERROR = "10", "XQuery error";
    CASE_NOT_FOUND_FOR_CASE_STATEMENT = "20", "case not found for case statement";
    CARDINALITY_VIOLATION = "21", "cardinality violation";
    DATA_EXCEPTION = "22", "data exception";
    INTEGRITY_CONSTRAINT_VIOLATION = "23", "integrity constraint violation";
    INVALID_CURSOR_STATE = "24", "invalid cursor state";
    INVALID_TRANSACTION_STATE = "25", "invalid transaction state";
    INVALID_SQL_STATEMENT_NAME = "26", "invalid SQL statement name";
    TRIGGERED_DATA_CHANGE_VIOLATION = "27", "triggered data change violation";
    INVALID_AUTHORIZATION_SPECIFICATION = "28", "invalid authorization specification";
    DEPENDENT_PRIVILEGE_DESCRIPTORS_STILL_EXIST = "2B", "dependent privilege descriptors still exist";
    INVALID_CHARACTER_SET_NAME = "2C", "invalid character set name";
    INVALID_TRANSACTION_TERMINATION = "2D", "invalid transaction termination";
    INVALID_CONNECTION_NAME = "2E", "invalid connection name";
    SQL_ROUTINE_EXCEPTION = "2F", "SQL routine exception";
    INVALID_COLLATION_NAME = "2H", "invalid collation name";
    INVALID_SQL_STATEMENT_IDENTIFIER = "30", "invalid SQL statement identifier";
    INVALID_SQL_DESCRIPTOR_NAME = "33", "invalid SQL descriptor name";
    INVALID_CURSOR_NAME = "34", "invalid cursor name";
    INVALID_CONDITION_NUMBER = "35", "invalid condition number";
    CURSOR_SENSITIVITY_EXCEPTION = "36", "cursor sensitivity exception";
    EXTERNAL_ROUTINE_EXCEPTION = "38", "external routine exception";
    EXTERNAL_ROUTINE_INVOCATION_EXCEPTION = "39", "external routine invocation exception";
    SAVEPOINT_EXCEPTION = "3B", "savepoint exception";
    AMBIGUOUS_CURSOR_NAME = "3C", "ambiguous cursor name";
    INVALID_CATALOG_NAME = "3D", "invalid catalog name";
    INVALID_SCHEMA_NAME = "3F", "invalid schema name";
    TRANSACTION_ROLLBACK = "40", "transaction rollback";
    SYNTAX_ERROR_OR_ACCESS_RULE_VIOLATION = "42", "syntax error or access rule violation";
    WITH_CHECK_OPTION_VIOLATION = "44", "with check option violation";
    UNHANDLED_USER_DEFINED_EXCEPTION = "45", "unhandled user-defined exception";
    OLB_SPECIFIC_ERROR = "46", "OLB-specific error";
    FDW_SPECIFIC_CONDITION = "HV", "FDW-specific condition";
    DATALINK_EXCEPTION = "HW", "datalink exception";
    CLI_SPECIFIC_CONDITION = "HY", "CLI-specific condition";
    REMOTE_DATABASE_ACCESS = "HZ", "remote database access";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_accepts_only_digits_and_uppercase_letters() {
        assert!(SqlState::new(*b"09AZ0").is_some());
        for invalid in [b'/', b':', b'@', b'[', b'a', b'z', b' ', 0, 0xC3] {
            let mut code = *b"23505";
            code[4] = invalid;
            assert_eq!(SqlState::new(code), None, "byte {invalid:#04x}");
        }
    }

    #[test]
    fn from_str_requires_exactly_five_bytes() {
        assert_eq!("23505".parse::<SqlState>().unwrap().as_str(), "23505");
        for invalid in ["", "2350", "235050", "2350\u{e9}", "2350\u{0}", "23505 "] {
            assert_eq!(
                invalid.parse::<SqlState>(),
                Err(InvalidSqlState),
                "{invalid:?}"
            );
        }
    }

    #[test]
    fn class_and_subclass_split_after_two_characters() {
        let code: SqlState = "40P01".parse().unwrap();
        assert_eq!(code.class(), SqlStateClass::TRANSACTION_ROLLBACK);
        assert_eq!(code.class().as_str(), "40");
        assert_eq!(code.subclass(), "P01");
    }

    #[test]
    fn standard_classes_start_with_0_to_4_or_a_to_h() {
        for code in [
            "00000", "40001", "0A000", "B0000", "G0000", "HV000", "H0000",
        ] {
            let code: SqlState = code.parse().unwrap();
            assert!(code.class().is_standard(), "{code}");
        }
        for code in ["50000", "9Z000", "I0000", "P0001", "XX000"] {
            let code: SqlState = code.parse().unwrap();
            assert!(!code.class().is_standard(), "{code}");
        }
    }

    #[test]
    fn constants_match_as_patterns() {
        const UNIQUE_VIOLATION: SqlState = SqlState::new(*b"23505").unwrap();
        let code: SqlState = "23505".parse().unwrap();
        assert!(matches!(code, UNIQUE_VIOLATION));
        assert!(matches!(
            code.class(),
            SqlStateClass::INTEGRITY_CONSTRAINT_VIOLATION
        ));
        let other: SqlState = "22012".parse().unwrap();
        assert!(!matches!(other, UNIQUE_VIOLATION));
        assert!(!matches!(
            other.class(),
            SqlStateClass::INTEGRITY_CONSTRAINT_VIOLATION
        ));
    }
}
