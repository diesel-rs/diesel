use backend::Backend;
use hlist::*;
use query_builder::*;
use query_source::Queryable;
use row::Row;
use types::{HasSqlType, FromSqlRow, Nullable, NotNull};

impl<Head, Tail, DB> HasSqlType<(Head, ...Tail)> for DB where
    DB: Backend + HasSqlType<Head> + HasSqlType<Tail>,
    Tail: Tuple,
{
    fn metadata() -> DB::TypeMetadata {
        unreachable!("hlists don't implement `ToSql` directly");
    }

    fn row_metadata(out: &mut Vec<DB::TypeMetadata>) {
        <DB as HasSqlType<Head>>::row_metadata(out);
        <DB as HasSqlType<Tail>>::row_metadata(out);
    }
}

impl<DB: Backend> HasSqlType<()> for DB {
    fn metadata() -> DB::TypeMetadata {
        unreachable!("hlists don't implement `ToSql` directly");
    }

    fn row_metadata(_: &mut Vec<DB::TypeMetadata>) {
        // noop
    }
}

impl<T: Tuple> NotNull for (...T) {
}

impl<Head, Tail, HeadST, TailST, DB> FromSqlRow<(HeadST, ...TailST), DB>
    for (Head, ...Tail) where
        Head: FromSqlRow<HeadST, DB>,
        Tail: FromSqlRow<TailST, DB> + Tuple,
        TailST: Tuple,
        DB: Backend + HasSqlType<HeadST> + HasSqlType<TailST>,
{
    fn build_from_row<R: Row<DB>>(row: &mut R) -> BuildQueryResult<Self> {
        Ok((
            try!(Head::build_from_row(row)),
            ...try!(Tail::build_from_row(row)),
        ))
    }
}

impl<DB: Backend> FromSqlRow<(), DB> for () {
    fn build_from_row<R: Row<DB>>(_: &mut R) -> BuildQueryResult<Self> {
        Ok(())
    }
}

impl<Head, Tail, ST, DB> FromSqlRow<Nullable<ST>, DB>
    for Option<(Head, ...Tail)> where
        DB: Backend + HasSqlType<ST>,
        (Head, ...Tail): FromSqlRow<ST, DB>,
        Tail: Tuple,
        ST: NotNull,
{
    fn build_from_row<R: Row<DB>>(row: &mut R) -> BuildQueryResult<Self> {
        if row.next_is_null((Head, ...Tail)::len()) {
            Ok(None)
        } else {
            (Head, ...Tail)::build_from_row(row).map(Some)
        }
    }
}

impl<Head, Tail, HeadST, TailST, DB> Queryable<(HeadST, ...TailST), DB>
    for (Head, ...Tail) where
        DB: Backend + HasSqlType<HeadST> + HasSqlType<TailST>,
        Head: Queryable<HeadST, DB>,
        Tail: Queryable<TailST, DB> + Tuple,
        <Tail as Queryable<TailST, DB>>::Row: Tuple,
        TailST: Tuple,
{
    type Row = (Head::Row, ...Tail::Row);

    fn build((head, ...tail): Self::Row) -> Self {
        (
            Head::build(head),
            ...Tail::build(tail),
        )
    }
}

impl<DB: Backend> Queryable<(), DB> for () {
    type Row = ();

    fn build(_: Self::Row) -> Self {
        ()
    }
}
