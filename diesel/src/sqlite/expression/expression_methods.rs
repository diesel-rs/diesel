use expression::Expression;
use super::collate::{Collate, Collation};

pub trait SqliteExpressionMethods: Expression + Sized {
    /// Creates SQLite `COLLATE` expression.
    /// https://sqlite.org/datatype3.html#collating_sequences
    fn collate<Coll: Collation>(self, collation: Coll) -> Collate<Self, Coll> {
        Collate::new(self, collation)
    }
}

impl<T: Expression> SqliteExpressionMethods for T {}
