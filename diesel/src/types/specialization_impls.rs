impl<T, ST, DB> FromSqlRow<Nullable<ST>, DB> for Option<T> where
    T: FromSqlRow<ST, DB>,
    DB: Backend + HasSqlType<ST>,
    ST: NotNull,
{
    default fn build_from_row<R: Row<DB>>(row: &mut R) -> Result<Self, Box<Error+Send+Sync>> {
        if row.next_is_null(1) {
            Ok(None)
        } else {
            T::build_from_row(row).map(Some)
        }
    }
}
