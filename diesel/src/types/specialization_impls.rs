impl<T, ST, DB> FromSqlRow<ST, DB> for T where
    T: FromSql<ST, DB>,
    DB: Backend + HasSqlType<ST>,
{
    default fn build_from_row<R: Row<DB>>(row: &mut R) -> Result<Self, Box<Error+Send+Sync>> {
        FromSql::<ST, DB>::from_sql(row.take())
    }
}
