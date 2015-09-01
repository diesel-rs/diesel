extern crate postgres;

use self::postgres::rows::Row as PgInnerRow;
use self::postgres::types::FromSql as PgFromSql;

pub trait Row {
    fn take<T: PgFromSql>(&mut self) -> T;
}

pub struct PgRow<'a> {
    inner: PgInnerRow<'a>,
    idx: usize,
}

impl<'a> PgRow<'a> {
    pub fn wrap(inner: PgInnerRow<'a>) -> Self {
        PgRow {
            inner: inner,
            idx: 0,
        }
    }
}

impl<'a> Row for PgRow<'a> {
    fn take<T: PgFromSql>(&mut self) -> T {
        let current_idx = self.idx;
        self.idx += 1;
        self.inner.get(current_idx)
    }
}
