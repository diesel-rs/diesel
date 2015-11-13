use connection::{Connection, Cursor};
use query_builder::AsQuery;
use query_source::Queriable;
use result::Result;
use types::NativeSqlType;

pub trait LoadDsl {
    type SqlType: NativeSqlType;

    fn load<U>(self, conn: &Connection) -> Result<Cursor<Self::SqlType, U>> where
        U: Queriable<Self::SqlType>;
}

impl<T: AsQuery> LoadDsl for T {
    type SqlType = <T as AsQuery>::SqlType;

    fn load<U>(self, conn: &Connection) -> Result<Cursor<Self::SqlType, U>> where
        U: Queriable<Self::SqlType>,
    {
        conn.query_all(self)
    }
}
