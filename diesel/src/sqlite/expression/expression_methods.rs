use expression::Expression;
use super::operators::*;

pub trait SqliteExpressionMethods: Sized {
    /// Creates SQLite `COLLATE BINARY` expression
    fn collate_binary(self) -> CollateBinary<Self> {
        CollateBinary::new(self)
    }

    /// Creates SQLite `COLLATE NOCASE` expression
    fn collate_nocase(self) -> CollateNoCase<Self> {
        CollateNoCase::new(self)
    }

    /// Creates SQLite `COLLATE RTRIM` expression
    fn collate_rtrim(self) -> CollateRTrim<Self> {
        CollateRTrim::new(self)
    }
}

impl<T: Expression> SqliteExpressionMethods for T {}
