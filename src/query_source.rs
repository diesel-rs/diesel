use super::types::{FromSql, NativeSqlType};

pub unsafe trait QuerySource {
    type SqlType: NativeSqlType;

    fn select_clause(&self) -> &str;
    fn from_clause(&self) -> &str;
}

pub trait Queriable<QS: QuerySource> {
    type Row: FromSql<QS::SqlType>;

    fn build(row: Self::Row) -> Self;
}
