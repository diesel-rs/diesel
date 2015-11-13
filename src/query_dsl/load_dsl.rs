use connection::{Connection, Cursor};
use query_builder::{Query, AsQuery};
use query_source::Queriable;
use result::Result;
use super::LimitDsl;

pub trait LoadDsl: AsQuery + LimitDsl + Sized {
    fn load<U>(self, conn: &Connection) -> Result<Cursor<Self::SqlType, U>> where
        U: Queriable<Self::SqlType>
    {
        conn.query_all(self)
    }

    fn first<U>(self, conn: &Connection) -> Result<Option<U>> where
        U: Queriable<<<Self as LimitDsl>::Output as Query>::SqlType>
    {
        conn.query_one(self.limit(1))
    }
}

impl<T: AsQuery + LimitDsl> LoadDsl for T {
}
