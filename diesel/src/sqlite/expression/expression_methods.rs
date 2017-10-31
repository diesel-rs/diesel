use expression::Expression;
use super::operators::*;

pub trait SqliteExpressionMethods: Sized {
    /// Creates SQLite `COLLATE BINARY` expression.
    /// https://sqlite.org/datatype3.html#collating_sequences
    fn collate_binary(self) -> CollateBinary<Self> {
        CollateBinary::new(self)
    }

    /// Creates SQLite `COLLATE NOCASE` expression.
    /// https://sqlite.org/datatype3.html#collating_sequences
    fn collate_nocase(self) -> CollateNoCase<Self> {
        CollateNoCase::new(self)
    }

    /// Creates SQLite `COLLATE RTRIM` expression.
    /// https://sqlite.org/datatype3.html#collating_sequences
    fn collate_rtrim(self) -> CollateRTrim<Self> {
        CollateRTrim::new(self)
    }
}

impl<T: Expression> SqliteExpressionMethods for T {}
